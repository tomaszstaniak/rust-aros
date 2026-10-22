#!/usr/bin/env python3
"""Build one private August std source tree with v11 and Jonx ARM backends.

Jonx files remain MIT OR Apache-2.0; copies keep their original content except
the documented native socket gate. Dispatch edits only select ARM-specific
modules. No upstream checkout, installed compiler or shared sysroot is edited.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
ARM = 'all(target_os = "aros", target_arch = "aarch64")'
VERSION = 'rustc 1.100.0-nightly (0dfb098f3 2026-08-31)'
spec = importlib.util.spec_from_file_location('jonx_sysroot', ROOT / 'tools/jonx-sysroot.py')
jonx = importlib.util.module_from_spec(spec)
spec.loader.exec_module(jonx)


def branch(text):
    """Extract the existing Jonx AROS cfg_select body, preserving nested blocks."""
    match = re.search(r'(?:target_os = "aros"|all\(target_os = "aros", not\(target_arch = "m68k"\)\)) => \{', text)
    if not match:
        raise ValueError('Jonx AROS dispatch arm missing')
    start = match.end()
    depth = 1
    for i in range(start, len(text)):
        depth += (text[i] == '{') - (text[i] == '}')
        if depth == 0:
            return text[start:i]
    raise ValueError('unterminated AROS dispatch arm')


def prepend(path, body):
    text = path.read_text()
    anchor = 'cfg_select! {'
    if anchor not in text:
        raise ValueError(f'cfg_select missing in {path}')
    path.write_text(text.replace(anchor, anchor + '\n    ' + ARM + ' => {' + body + '\n    }', 1))


def merge(std, source):
    # Import the actual Jonx implementation, not a second reconstruction.
    js = source / 'library/std/src'
    # Cargo compiles std's libc dependency even when this non-unix ARM PAL
    # never uses it. Do not compile v11 C glue or expose v11 bindings on ARM.
    libc = std.parents[1] / 'libc-aros'
    binding = libc / 'src/lib.rs'
    text = binding.read_text()
    start = text.index('#[cfg(feature = "rustc-dep-of-std")]')
    body = text[start:].replace('mod terminal;', '#[path = "terminal.rs"]\nmod terminal;', 1)
    alias = '#[cfg(feature = "rustc-dep-of-std")]\nextern crate rustc_std_workspace_core as core;'
    body = body.replace(alias, '#[cfg(feature = "rustc-dep-of-std")]\nuse crate::core;', 1)
    (libc / 'src/v11.rs').write_text(body)
    binding.write_text(text[:start] + alias + '\n#[cfg(not(' + ARM + '))]\nmod v11;\n' +
                       '#[cfg(not(' + ARM + '))]\npub use v11::*;\n')
    build = libc / 'build.rs'
    build.write_text(build.read_text().replace('fn main() {',
                     'fn main() {\n    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64") { return; }', 1))
    for src in (js / 'sys').rglob('aros.rs'):
        dest = std / src.relative_to(js)
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)
    shutil.copytree(js / 'sys/pal/unsupported/sync', std / 'sys/pal/unsupported/sync')
    for name in ['alloc', 'args', 'env', 'fs', 'io/error', 'net/connection',
                 'path', 'paths', 'pipe', 'process', 'random', 'stdio', 'thread', 'time']:
        rel = Path('sys') / name / 'mod.rs'
        body = branch((js / rel).read_text())
        if name == 'alloc':
            body += '\n        use aros as imp;'
        prepend(std / rel, body)
    # Internal API adaptations and the native socket gate are reviewable,
    # versioned patches against the pinned Jonx source.
    subprocess.run(['patch', '-p1', '-i', ROOT / 'patches/jonx/0001-rust-1.100-native-contracts.patch'],
                   cwd=std, check=True)
    prepend(std / 'sys/pal/mod.rs', '\n        mod unsupported;\n        pub use unsupported::*;')
    with (std / 'sys/pal/unsupported/mod.rs').open('a') as f:
        f.write('\n#[cfg(' + ARM + ')]\npub mod sync;\n')
    for name in ['mutex', 'condvar', 'rwlock', 'thread_parking']:
        module, item = {'mutex': ('pthread', 'Mutex'), 'condvar': ('pthread', 'Condvar'),
                        'rwlock': ('queue', 'RwLock'), 'thread_parking': ('pthread', 'Parker')}[name]
        prepend(std / f'sys/sync/{name}/mod.rs', f'\n        mod {module};\n        pub use {module}::{item};')
    prepend(std / 'sys/sync/once/mod.rs', '\n        mod queue;\n        pub use queue::{Once, OnceState};')
    # AROS's key-based TLS branch is independent of the surrounding compiler
    # version. Keep August's other TLS implementations intact.
    tls = std / 'sys/thread_local/mod.rs'
    text = tls.read_text()
    text = text.replace('pub(crate) mod guard {\n    cfg_select! {',
                        'pub(crate) mod guard {\n    cfg_select! {\n        ' + ARM + ' => { pub(crate) fn enable() {} }', 1)
    key = text.index('pub(crate) mod key {')
    before, after = text[:key], text[key:]
    after = after.replace('cfg_select! {', 'cfg_select! {\n        ' + ARM + ' => {' +
                          branch((js / 'sys/thread_local/mod.rs').read_text()) + '\n        }', 1)
    tls.write_text(before + after)
    env = std / 'sys/env/mod.rs'
    env.write_text(env.read_text().replace('    target_family = "unix",', '    ' + ARM + ',\n    target_family = "unix",', 1))
    # Keep existing v11 OS extensions; select the imported fd/AROS extensions
    # only on ARM, where the fs implementation is Jonx's C bridge.
    shutil.copytree(js / 'os/aros', std / 'os/aros_jonx')
    shutil.copytree(js / 'os/fd', std / 'os/fd_jonx')
    osmod = std / 'os/mod.rs'
    text = osmod.read_text().replace('    unix,', '    ' + ARM + ',\n    unix,', 1)
    text = text.replace('pub mod fd;', '#[cfg_attr(' + ARM + ', path = "fd_jonx/mod.rs")]\npub mod fd;', 1)
    text = text.replace('pub mod aros;', '#[cfg_attr(' + ARM + ', path = "aros_jonx/mod.rs")]\npub mod aros;', 1)
    osmod.write_text(text)
    exitfile = std / 'sys/exit.rs'
    text = exitfile.read_text()
    marker = 'pub fn exit(code: i32) -> ! {\n    cfg_select! {'
    text = text.replace(marker, marker + '\n        ' + ARM + ' => {\n            unsafe extern "C" { fn exit(code: crate::ffi::c_int) -> !; }\n            unsafe { exit(code) }\n        }', 1)
    exitfile.write_text(text)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('jonx_source', type=Path)
    p.add_argument('new_sysroot', type=Path)
    p.add_argument('--base', default='nightly-2026-09-01', help='installed compiler, checked against the exact pinned version')
    a = p.parse_args()
    source, dest = a.jonx_source.resolve(), a.new_sysroot.absolute()
    jonx.validate_source(source)
    base = Path(subprocess.check_output(['rustup', 'run', a.base, 'rustc', '--print', 'sysroot'], text=True).strip())
    version = subprocess.check_output([base / 'bin/rustc', '-V'], text=True).strip()
    if version != VERSION:
        p.error(f'base mismatch: {version}; require {VERSION}')
    jonx.create_sysroot(base, base / 'lib/rustlib/src/rust', dest)
    try:
        populate(dest, source)
    except BaseException:
        shutil.rmtree(dest)
        raise
    print(dest)


def populate(dest, source):
    lib = dest / 'lib/rustlib/src/rust/library'
    shutil.copytree(ROOT / 'std-groundwork/libc-aros', lib / 'libc-aros',
                    ignore=shutil.ignore_patterns('target', '__pycache__'))
    cargo = lib / 'Cargo.toml'
    cargo.write_text(cargo.read_text().replace('[patch.crates-io]\n',
                     '[patch.crates-io]\nlibc = { path = "libc-aros" }\n', 1))
    subprocess.run(['python3', ROOT / 'std-groundwork/std-patch/patch-std.py', lib / 'std/src'], check=True)
    merge(lib / 'std/src', source)
    (dest / 'aros-common.json').write_text(json.dumps(dict(compiler=VERSION, jonx=jonx.REVISION,
         backends=['x86_64-v11', 'aarch64-v1-raspi'], native_sockets='disabled'), indent=2) + '\n')


if __name__ == '__main__':
    main()
