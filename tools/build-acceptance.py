#!/usr/bin/env python3
"""Build the same std acceptance probe for explicit AROS CPU/ABI/SDK inputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sysroot_contract

ROOT = Path(__file__).resolve().parents[1]
GLUE_REV = '3bc9359d439f9fb5655cf48924413ffe670931cc'


def validate_elf(data, profile):
    abi, machine = (1, 183) if profile == 'aarch64-v1-raspi' else (11, 62)
    if (len(data) < 64 or data[:8] != bytes.fromhex('7f454c460201010f') or data[8] != abi
            or int.from_bytes(data[16:18], 'little') != 1
            or int.from_bytes(data[18:20], 'little') != machine):
        raise ValueError(f'ELF class/endian/OSABI/ABI/type/machine mismatch for {profile}')


def tree_digest(root):
    digest = hashlib.sha256()
    for path in sorted(root.rglob('*')):
        if path.is_file():
            digest.update(path.relative_to(root).as_posix().encode() + b'\0')
            digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def run(args, *, env=None, cwd=ROOT):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), check=True, env=env, cwd=cwd, timeout=600)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--profile', choices=['aarch64-v1-raspi', 'x86_64-v11'], required=True)
    for name in ['sysroot', 'cc', 'ar', 'nm', 'sdk', 'out']:
        p.add_argument('--' + name, type=Path, required=True)
    p.add_argument('--glue', type=Path, help='clean pinned pomaros-hosted checkout (ARM only)')
    a = p.parse_args()
    for name in ['sysroot', 'cc', 'ar', 'nm', 'sdk', 'out']:
        setattr(a, name, getattr(a, name).absolute())
    arm = a.profile == 'aarch64-v1-raspi'
    try:
        sysroot_contract.validate(a.sysroot, a.profile)
    except ValueError as error:
        p.error(f'{error}; create the guarded common sysroot with tools/common-sysroot.py')
    expected = 'rustc 1.100.0-nightly (0dfb098f3 2026-08-31)'
    rustc = a.sysroot / 'bin/rustc'
    version = subprocess.check_output([rustc, '-V'], text=True).strip()
    if version != expected:
        p.error(f'compiler mismatch: {version}; require {expected}')
    if any(os.environ.get(key) for key in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']):
        p.error('clear ambient Rust flags/wrappers; profiles own the compiler contract')
    if arm:
        if not a.glue:
            p.error('--glue must name the pinned pomaros-hosted checkout')
        a.glue = a.glue.resolve()
        rev = subprocess.check_output(['git', '-C', a.glue, 'rev-parse', 'HEAD'], text=True).strip()
        dirty = subprocess.check_output(['git', '-C', a.glue, 'status', '--porcelain'], text=True)
        if rev != GLUE_REV or dirty:
            p.error(f'glue must be clean at {GLUE_REV}')
    a.out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, RUSTC=str(rustc), AROS_GCC=str(a.cc), AROS_AR=str(a.ar),
               AROS_SDK=str(a.sdk), RPI_SDK=str(a.sdk))
    # cargo must use the wrapper for host build scripts/proc macros as well.
    env['PATH'] = str(a.sysroot / 'bin') + os.pathsep + env.get('PATH', '')
    if arm:
        target = json.loads((ROOT / 'profiles/aarch64-v1-raspi.json').read_text())
    else:
        template = (ROOT / 'x86_64-aros.json.in').read_text()
        template = template.replace('@AROS_GCC@', 'CC').replace('@AROS_SDK@', 'SDK').replace('@AROS_EXTRA_LIBS@', '')
        target = json.loads(template)
        target['linker'] = str(a.cc)
        target['pre-link-args']['gnu-cc'] = ['--sysroot=' + str(a.sdk)]
    spec = a.out / (a.profile + '.json')
    spec.write_text(json.dumps(target, indent=2) + '\n')
    cargo = [a.sysroot / 'bin/cargo', 'build', '--lib', '--offline', '--release',
             '-Zjson-target-spec', '-Zbuild-std=std,panic_abort', '--manifest-path', ROOT / 'acceptance/Cargo.toml',
             '--target', spec, '--target-dir', a.out / 'cargo']
    run(cargo, env=env)
    sources = [ROOT / 'acceptance/main.c']
    if arm:
        sources += [ROOT / 'acceptance/sdk-contract.c']
        sources += [a.glue / 'hosted/rust' / (name + '.c') for name in
                    ['aros_fs_glue', 'aros_thread_glue', 'aros_sync_glue', 'aros_env_glue',
                     'aros_process_glue', 'aros_proc_glue', 'aros_net_glue']]
    objects = []
    for source in sources:
        obj = a.out / (source.stem + '.o')
        run([a.cc, '--sysroot=' + str(a.sdk), '-O2', '-c', source, '-o', obj], env=env)
        objects.append(obj)
    archive = a.out / 'cargo' / a.profile / 'release/libaros_acceptance.a'
    binary = a.out / 'aros-std-acceptance'
    # A C-driven staticlib link must carry Rust's native dynamic link metadata
    # explicitly. Preserve libc-aros's crt-before-net strerror ordering.
    libraries = ['-lpthread'] if arm else ['-lcrt', '-lnet', '-lpthread']
    run([a.cc, '--sysroot=' + str(a.sdk), *objects, archive, *libraries,
         '-Wl,--no-gc-sections', '-o', binary], env=env)
    undefined = subprocess.check_output([a.nm, '--undefined-only', binary], text=True)
    (a.out / 'undefined.txt').write_text(undefined)
    if undefined.strip():
        sys.exit('unresolved symbols: ' + undefined)
    data = binary.read_bytes()
    validate_elf(data, a.profile)
    manifest = dict(profile=a.profile, compiler=version, sdk=str(a.sdk), cc=str(a.cc), ar=str(a.ar), nm=str(a.nm),
                    cc_version=subprocess.check_output([a.cc, '--version'], text=True),
                    target=target, glue_revision=GLUE_REV if arm else None,
                    sha256=hashlib.sha256(data).hexdigest(), runtime_acceptance=False)
    manifest['sdk_sha256'] = tree_digest(a.sdk)
    manifest['std_source_sha256'] = tree_digest(a.sysroot / 'lib/rustlib/src/rust/library/std')
    manifest['cc_sha256'] = hashlib.sha256(a.cc.read_bytes()).hexdigest()
    # Build the preceding acceptance gates with the same compiler/SDK pair.
    run([a.cc, '--sysroot=' + str(a.sdk), '-O2', '-DAROS_C_PROBE',
         ROOT / 'acceptance/main.c', '-o', a.out / 'aros-c-acceptance'], env=env)
    nostd = list(cargo)
    nostd[nostd.index('-Zbuild-std=std,panic_abort')] = '-Zbuild-std=core'
    nostd[nostd.index('--manifest-path') + 1] = ROOT / 'acceptance/nostd/Cargo.toml'
    run(nostd, env=env)
    run([a.cc, '--sysroot=' + str(a.sdk), a.out / 'main.o',
         a.out / 'cargo' / a.profile / 'release/libaros_nostd_acceptance.a',
         '-Wl,--no-gc-sections', '-o', a.out / 'aros-nostd-acceptance'], env=env)
    for name in ['aros-c-acceptance', 'aros-nostd-acceptance']:
        validate_elf((a.out / name).read_bytes(), a.profile)
        if subprocess.check_output([a.nm, '--undefined-only', a.out / name], text=True).strip():
            sys.exit(f'unresolved symbols in {name}')
    manifest['artifacts'] = {name: hashlib.sha256((a.out / name).read_bytes()).hexdigest()
                             for name in ['aros-c-acceptance', 'aros-nostd-acceptance', 'aros-std-acceptance']}
    (a.out / 'build.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(f'Built and linked {binary}; runtime acceptance NOT performed')


if __name__ == '__main__':
    main()
