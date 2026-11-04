#!/usr/bin/env python3
"""Prepare private headers for the pinned mainline pthread2 contract.

Import libraries remain from the named base SDK. This composition is not a
built runtime and grants no execution qualification. All input SDK files stay
untouched. New positioned imports are generated separately from posixc.conf.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('sdk', 'source', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    output = a.output.resolve()
    protected = [a.sdk.resolve(), a.source.resolve()]
    if any(output == root or root in output.parents for root in protected):
        p.error('output must be new and outside source/SDK inputs')
    output.mkdir(parents=True, exist_ok=False)
    originals = {str(x): digest(x) for directory in ('include', 'lib') for x in
                 (a.sdk.resolve() / directory).rglob('*') if x.is_file()}
    for name in ('include', 'lib'):
        shutil.copytree(a.sdk / name, output / name)
    adaptations = {}
    headers = {name: 'compiler/pthread/' + name for name in ('pthread.h', 'semaphore.h', 'sched.h')}
    headers['sys/cdefs.h'] = 'workbench/network/common/include/sys/cdefs.h'
    for name, relative in headers.items():
        source = a.source.resolve() / relative
        adaptations[str(source)] = digest(source)
        shutil.copyfile(source, output / 'include' / name)
    if any(digest(Path(name)) != expected for name, expected in originals.items()):
        raise RuntimeError('base SDK changed during composition')
    (output / 'sdk-composition.json').write_text(json.dumps(dict(schema=1,
        original_inputs=originals, header_inputs=adaptations,
        runtime='UNTESTED', import_libraries='base SDK; not rebuilt residents'), indent=2) + '\n')


if __name__ == '__main__':
    main()
