#!/usr/bin/env python3
"""Build existing acceptance workloads with an installed Jonx official SDK.

This is a consumer of integrate.py, not another std/sysroot implementation.
All execution gates remain UNTESTED; guest shell receipts are mandatory.
"""
import argparse
import hashlib
import importlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('source', 'install-prefix', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    here = a.source.resolve() / 'tools/crosstools/rust/unmodified'
    sys.path.insert(0, str(here))
    profiles = importlib.import_module('profiles')
    pin = json.loads((here / 'channel.json').read_text())
    prefix = a.install_prefix.resolve()
    installed = profiles.validate_installation(prefix, pin['compiler_commit'])
    native = Path(installed['native_build'])
    result = json.loads((native / 'result.json').read_text())
    profile = installed['profile']; triple = profile['triple']
    output = a.output.resolve()
    protected = (ROOT, a.source.resolve(), prefix, native, Path(result['sdk']))
    if any(output == root or root in output.parents for root in protected):
        p.error('output must be new and outside source/SDK trees')
    output.mkdir(parents=True, exist_ok=False)
    environment = {k: v for k, v in os.environ.items() if not k.startswith('CARGO_') and k not in
                   ('RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'RUSTC_BOOTSTRAP')}
    environment.update(RUSTC=str(prefix/'bin/rustc'), CARGO_HOME=str(output/'cargo-home'),
                       CARGO_TARGET_DIR=str(output/'target'), CARGO_INCREMENTAL='0')
    commands = []
    def run(command, name):
        commands.append([str(x) for x in command])
        (output/(name+'-command.json')).write_text(json.dumps(commands[-1], indent=2)+'\n')
        with (output/(name+'.log')).open('w') as log:
            subprocess.run(commands[-1], env=environment, stdout=log, stderr=subprocess.STDOUT,
                           check=True, timeout=300, cwd=output)
    cargo = [prefix/'bin/cargo', '--config', prefix/'cargo-config.toml', '--config',
             'profile.dev.panic="abort"', 'build', '--offline', '--jobs', '2']
    run([*cargo, '--manifest-path', ROOT/'acceptance/Cargo.toml', '--bin', 'aros-acceptance'], 'std-main')
    run([*cargo, '--manifest-path', ROOT/'acceptance/nostd/Cargo.toml'], 'nostd')
    products = output/'programs'; products.mkdir()
    shutil.copyfile(output/'target'/triple/'debug/aros-acceptance', products/'std')
    cc = [str(prefix/'bin'/profile['driver'])] + ['-fno-pic', '-mcmodel=large', '-O2', '-Wl,--no-gc-sections']
    harness = ROOT/'acceptance/main.c'
    run([*cc, '-DAROS_C_PROBE', harness, '-o', products/'c'], 'c')
    run([*cc, harness, output/'target'/triple/'debug/libaros_nostd_acceptance.a', '-o', products/'nostd'], 'nostd-link')
    for binary in products.iterdir():
        data = binary.read_bytes()
        if data[:6] != b'\x7fELF\x02\x01' or data[7:9] != bytes([15, 1]) or int.from_bytes(data[18:20], 'little') != profile['machine']:
            raise ValueError('acceptance binary differs from selected CPU/ABI')
    profiles.validate_installation(prefix, pin['compiler_commit'])
    manifest = dict(schema=1, profile=profile, generation=installed['generation'],
                    compiler=pin['compiler_commit'], native_runtime='UNTESTED',
                    artifacts={name: digest(products/name) for name in ('c', 'nostd', 'std')},
                    source_inputs={str(x): digest(x) for x in [Path(__file__).resolve(), ROOT/'acceptance/Cargo.toml', ROOT/'acceptance/nostd/Cargo.toml', harness, *sorted((ROOT/'acceptance/src').glob('*.rs')),
                                                              ROOT/'acceptance/nostd/src/lib.rs']},
                    commands=commands)
    for name in ('c', 'nostd', 'std'):
        shutil.copyfile(products/name, output/('aros-'+name+'-acceptance'))
    (output/'build.json').write_text(json.dumps(dict(profile=triple+'-ABIv1-official',
        compiler=pin['compiler_commit'], native_runtime='UNTESTED',
        artifacts={'aros-'+name+'-acceptance': manifest['artifacts'][name] for name in ('c', 'nostd', 'std')}), indent=2)+'\n')
    (output/'result.json').write_text(json.dumps(manifest, indent=2)+'\n')
    print('PASS C/no_std/ordinary Cargo-main build/link; runtime UNTESTED')


if __name__ == '__main__':
    main()
