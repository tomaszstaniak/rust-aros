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
    p.add_argument('--cargo-main', action='store_true', help='ordinary Rust main: workload/ExitCode 7, repeated three times')
    a = p.parse_args()
    for value in [a.programs, a.results, a.run_id]:
        if not re.fullmatch(r'[A-Za-z0-9_:/.-]+', value):
            p.error('use simple ASCII volume/drawer names without whitespace or shell syntax')
    if not re.fullmatch(r'[A-Za-z0-9_-]+', a.run_id):
        p.error('run-id must not contain a path')
    manifest = json.loads((a.build / 'build.json').read_text())
    names = {f'aros-{kind}-acceptance' for kind in ('c', 'nostd', 'std')}
    artifacts = manifest.get('artifacts')
    if not isinstance(artifacts, dict) or set(artifacts) != names:
        p.error('artifacts must contain exactly the C, no_std and std programs')
    # Keep the verified bytes: reopening a source during copying would allow a
    # changed file to bypass verification between the check and the copy.
    payloads = {}
    for name in sorted(names):
        expected = artifacts[name]
        if not isinstance(expected, str) or not re.fullmatch(r'[0-9a-f]{64}', expected):
            p.error(f'invalid SHA-256: {name}')
        data = (a.build / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != expected:
            p.error(f'artifact hash mismatch: {name}')
        payloads[name] = data
    for field in ('profile', 'compiler'):
        if not isinstance(manifest.get(field), str) or not manifest[field]:
            p.error(f'missing or invalid {field}')
    a.new_bundle.mkdir(parents=True, exist_ok=False)
    try:
        shutil.copy2(a.build / 'build.json', a.new_bundle / 'build.json')
        for kind in ['c', 'nostd', 'std']:
            target = a.new_bundle / kind
            target.write_bytes(payloads[f'aros-{kind}-acceptance'])
            target.chmod((a.build / f'aros-{kind}-acceptance').stat().st_mode & 0o777)
        def join(drawer, name):
            return drawer + ('' if drawer.endswith((':', '/')) else '/') + name
        lines = ['FailAt 21', 'Stack 4194304']
        cases = [('c', '', 0), ('nostd', '', 0)]
        cases += ([('std', ' runtime-argv workload', 0), ('std', ' runtime-argv exit7', 7)] * 3 if a.cargo_main
                  else [('std', '', 0)] * 3)
        for n, (kind, arguments, expected_rc) in enumerate(cases, 1):
            result = join(a.results, f'{a.run_id}-{n}-{kind}')
            lines += [f'{join(a.programs, kind)}{arguments} >{result}.log', f'Echo $RC >{result}.rc']
        lines += [f'Echo complete >{join(a.results, a.run_id + "-done.txt")}']
        (a.new_bundle / 'expected-receipts.json').write_text(json.dumps(dict(run_id=a.run_id,
            exit_codes=[code for _, _, code in cases]), indent=2) + '\n')
        (a.new_bundle / 'run').write_text('\n'.join(lines) + '\n')
        shutil.copy2(Path(__file__).resolve().parents[1] / 'acceptance/README.md', a.new_bundle / 'README.md')
        (a.new_bundle / 'RUN.txt').write_text(
            f'Profile: {manifest["profile"]}\nCompiler: {manifest["compiler"]}\n'
            f'Copy this bundle into {a.programs}. Ensure {a.results} exists.\n'
            f'Run Execute {join(a.programs, "run")} with a 45-second host deadline.\n'
            f'Require fresh {a.run_id}-*.rc receipts matching expected-receipts.json and the final complete receipt.\n'
            'Work-complete output without a return-code receipt is not success.\n'
            'Physical Pi model, image compatibility and runtime acceptance must be verified separately.\n')
    except BaseException:
        shutil.rmtree(a.new_bundle)
        raise
    print(a.new_bundle)


if __name__ == '__main__':
    main()
