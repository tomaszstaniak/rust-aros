#!/usr/bin/env python3
"""Build the separate Rust-main runtime gate using an existing acceptance build."""
import argparse
import hashlib
import json
import os
import shlex
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sysroot_contract
import importlib.util

_spec = importlib.util.spec_from_file_location(
    "acceptance_builder", Path(__file__).with_name("build-acceptance.py"))
builder = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(builder)

ROOT = Path(__file__).resolve().parents[1]


def write_linker(path, compiler):
    path.write_text('#!/bin/sh\nset -e\nexec ' + shlex.quote(str(compiler)) +
                    ' "$@" -Wl,--no-gc-sections\n')
    path.chmod(0o755)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('build', type=Path)
    parser.add_argument('sysroot', type=Path)
    parser.add_argument('new_out', type=Path)
    args = parser.parse_args()
    build, sysroot, out = args.build.absolute(), args.sysroot.absolute(), args.new_out.absolute()
    manifest = json.loads((build / 'build.json').read_text())
    profile = manifest['profile']
    contract = sysroot_contract.validate(sysroot, profile)
    if builder.tree_digest(Path(manifest['sdk'])) != manifest['sdk_sha256']:
        parser.error('SDK differs from the acceptance build')
    compiler = subprocess.check_output([sysroot / 'bin/rustc', '-V'], text=True).strip()
    if compiler != manifest['compiler']:
        parser.error('compiler differs from the acceptance build')
    arm = profile == 'aarch64-v1-raspi'
    target = json.loads((build / f'{profile}.json').read_text())
    target['no-default-libraries'] = False
    target['linker'] = str(out / 'linker')
    target['pre-link-args'] = {'gnu-cc': ['--sysroot=' + manifest['sdk']]}
    if arm:
        objects = sorted(build.glob('aros_*_glue.o'))
        if len(objects) != 7:
            parser.error('require all seven bridge objects from the acceptance build')
        target['pre-link-args']['gnu-cc'] += list(map(str, objects))
    target['late-link-args'] = {'gnu-cc': ['-lpthread'] if arm else ['-lcrt', '-lnet', '-lpthread']}
    out.mkdir(parents=True, exist_ok=False)
    # Apply this only at final link, without compiling unused architecture
    # intrinsics as -Clink-dead-code would do when rebuilding core.
    linker = out / 'linker'
    write_linker(linker, manifest['cc'])
    spec = out / f'{profile}.json'
    spec.write_text(json.dumps(target, indent=2) + '\n')
    env = dict(os.environ, RUSTC=str(sysroot / 'bin/rustc'), AROS_GCC=manifest['cc'],
               AROS_AR=manifest['ar'], AROS_SDK=manifest['sdk'], RPI_SDK=manifest['sdk'])
    if any(os.environ.get(key) for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER')):
        parser.error('clear ambient Rust flags and wrappers')
    env['PATH'] = str(sysroot / 'bin') + os.pathsep + env.get('PATH', '')
    subprocess.run([sysroot / 'bin/cargo', 'build', '--bin', 'aros-acceptance', '--offline',
                    '--release', '-Zjson-target-spec', '-Zbuild-std=std,panic_abort',
                    '--manifest-path', ROOT / 'acceptance/Cargo.toml', '--target', spec,
                    '--target-dir', out / 'cargo'], env=env, check=True, timeout=600)
    binary = out / 'cargo' / profile / 'release/aros-acceptance'
    data = binary.read_bytes()
    builder.validate_elf(data, profile)
    undefined = subprocess.check_output([manifest['nm'], '-u', binary], text=True).strip()
    symbols = subprocess.check_output([manifest['nm'], binary], text=True)
    if undefined or not any(line.endswith(' T main') for line in symbols.splitlines()):
        parser.error('require a defined main and no unresolved symbols')
    (out / 'cargo-main').write_bytes(data)
    (out / 'cargo-main').chmod(0o755)
    manifest.pop('artifacts', None)
    manifest.update(sysroot_contract=contract, target=target,
                    std_sha256=builder.tree_digest(sysroot / sysroot_contract.SOURCE / 'library/std'),
                    linker_sha256=hashlib.sha256(linker.read_bytes()).hexdigest(),
                    bridge_objects={str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in (objects if arm else [])},
                    gate='cargo-rust-main', sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                    runtime_acceptance=False)
    (out / 'build.json').write_text(json.dumps(manifest, indent=2) + '\n')


if __name__ == '__main__':
    main()
