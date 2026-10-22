#!/usr/bin/env python3
"""Create a private test bundle; never deploy to a disk or running system."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('build', type=Path)
    p.add_argument('new_bundle', type=Path)
    p.add_argument('--programs', required=True, help='guest drawer after copying, e.g. RAM:rustprobe')
    p.add_argument('--results', required=True, help='existing private guest result drawer, e.g. RESULTS:')
    p.add_argument('--run-id', required=True, help='unique short prefix for fresh result files')
    a = p.parse_args()
    for value in [a.programs, a.results, a.run_id]:
        if not re.fullmatch(r'[A-Za-z0-9_:/.-]+', value):
            p.error('use simple ASCII volume/drawer names without whitespace or shell syntax')
    if not re.fullmatch(r'[A-Za-z0-9_-]+', a.run_id):
        p.error('run-id must not contain a path')
    manifest = json.loads((a.build / 'build.json').read_text())
    for name, expected in manifest['artifacts'].items():
        if hashlib.sha256((a.build / name).read_bytes()).hexdigest() != expected:
            p.error(f'artifact hash mismatch: {name}')
    a.new_bundle.mkdir(parents=True, exist_ok=False)
    shutil.copy2(a.build / 'build.json', a.new_bundle / 'build.json')
    for kind in ['c', 'nostd', 'std']:
        shutil.copy2(a.build / f'aros-{kind}-acceptance', a.new_bundle / kind)
    def join(drawer, name):
        return drawer + ('' if drawer.endswith((':', '/')) else '/') + name
    lines = ['FailAt 21', 'Stack 4194304']
    for n, kind in enumerate(['c', 'nostd', 'std', 'std', 'std'], 1):
        result = join(a.results, f'{a.run_id}-{n}-{kind}')
        lines += [f'{join(a.programs, kind)} >{result}.log', f'Echo $RC >{result}.rc']
    lines += [f'Echo complete >{join(a.results, a.run_id + "-done.txt")}']
    (a.new_bundle / 'run').write_text('\n'.join(lines) + '\n')
    print(a.new_bundle)


if __name__ == '__main__':
    main()
