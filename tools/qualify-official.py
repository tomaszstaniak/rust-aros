#!/usr/bin/env python3
"""Run host/build/link qualification through Jonx's existing official pipeline.

This driver does not implement std construction, SDK installation or ABI rules.
It invokes the pinned implementations and retains receipts. No VM is started.
"""
import argparse
import json
from pathlib import Path
import subprocess
import shutil
import time

ROOT = Path(__file__).resolve().parents[1]


def retain_bundle_metadata(output, sdk):
    manifest = json.loads((output/'sdk/aros-official-rust.json').read_text())
    native = Path(manifest['native_build'])
    for source, name in ((output/'sdk/aros-official-rust.json', 'sdk-installation.json'),
                         (native/'result.json', 'native-build.json'),
                         (sdk/'sdk-composition.json', 'sdk-composition.json'),
                         (output/'baseline/result.json', 'baseline-result.json'),
                         (ROOT/'profiles/mainline-official/sources.json', 'sources.json')):
        shutil.copyfile(source, output/'bundle'/name)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--target', choices=('x86_64-unknown-aros', 'aarch64-unknown-aros'), required=True)
    for name in ('source', 'host-root', 'rust-src', 'sdk', 'crosstools', 'collect-aros',
                 'posix-stubs', 'application-libc', 'output'):
        p.add_argument('--'+name, type=Path, required=True)
    p.add_argument('--std-build', type=Path)
    p.add_argument('--cc', type=Path)
    p.add_argument('--cc-arg', action='append', default=[])
    p.add_argument('--host-triple', required=True)
    p.add_argument('--run-id', required=True, help='fresh ASCII receipt prefix for the future native run')
    a = p.parse_args(); output = a.output.resolve()
    protected = [a.source, a.host_root, a.rust_src, a.sdk, a.crosstools, a.application_libc]
    if any(output == root.resolve() or root.resolve() in output.parents for root in protected):
        p.error('output must be new and outside input trees')
    output.mkdir(parents=True, exist_ok=False)
    here = a.source.resolve()/'tools/crosstools/rust/unmodified'
    records = []
    def run(command, name):
        command = [str(x) for x in command]
        (output/(name+'-command.json')).write_text(json.dumps(command, indent=2)+'\n')
        started = time.monotonic()
        with (output/(name+'.log')).open('w') as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=600)
        records.append(dict(step=name, rc=result.returncode, elapsed=time.monotonic()-started))
        (output/'steps.json').write_text(json.dumps(records, indent=2)+'\n')
        result.check_returncode()
    install = ['python3', here/'integrate.py', '--target', a.target, '--prefix', output/'sdk',
               '--work', output/'install', '--host-root', a.host_root.resolve(), '--host-triple', a.host_triple,
               '--rust-src', a.rust_src.resolve(), '--sdk', a.sdk.resolve(), '--crosstools', a.crosstools.resolve(),
               '--collect-aros', a.collect_aros.resolve(), '--posix-stubs', a.posix_stubs.resolve(), '--offline', '--jobs', '2']
    if a.std_build: install += ['--std-build', a.std_build.resolve()]
    if a.cc: install += ['--cc', a.cc.resolve()]
    install += ['--cc-arg='+flag for flag in a.cc_arg]
    run(install, 'install')
    native = Path(json.loads((output/'sdk/aros-official-rust.json').read_text())['native_build'])
    for suffix, extra in (('consumer', []), ('consumer-fat', ['--lto', 'fat']), ('consumer-thin', ['--lto', 'thin'])):
        run(['python3', here/'consumer-check.py', '--install-prefix', output/'sdk', '--application-libc',
             a.application_libc.resolve(), '--output', output/suffix, *extra], suffix)
    libs = list((output/'consumer/target'/a.target).rglob('liblibc-*.rlib'))
    if len(libs) != 1: raise ValueError('missing unique application libc artifact')
    for name, extra in (('abi-std', []), ('abi-application', ['--application-libc-rlib', libs[0]])):
        run(['python3', here/'ABI/network-abi.py', '--sdk', a.sdk.resolve(), '--crosstools', a.crosstools.resolve(),
             '--std-build', native, '--output', output/name, *extra], name)
    run(['python3', here/'ABI/libc-exports.py', '--std-build', native, '--application-libc-rlib', libs[0],
         '--crosstools', a.crosstools.resolve(), '--binary', output/'consumer/target'/a.target/'debug/aros-precompiled-std-consumer',
         '--output', output/'exports'], 'exports')
    run(['python3', ROOT/'tools/official-acceptance.py', '--source', a.source.resolve(),
         '--install-prefix', output/'sdk', '--output', output/'baseline'], 'baseline')
    run(['python3', ROOT/'tools/package-acceptance.py', output/'baseline', output/'bundle', '--cargo-main',
         '--programs', 'RAM:RUSTV1', '--results', 'RESULTS:', '--run-id', a.run_id], 'package')
    retain_bundle_metadata(output, a.sdk.resolve())
    (output/'result.json').write_text(json.dumps(dict(target=a.target, build_link='PASS',
        runtime='UNTESTED', steps=records), indent=2)+'\n')
    print('PASS official SDK host/build/link gates; native runtime UNTESTED')


if __name__ == '__main__':
    main()
