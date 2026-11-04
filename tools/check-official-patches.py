#!/usr/bin/env python3
"""Replay the versioned adaptation series against pinned PR1800 preimages."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    source = a.source.resolve(); output = a.output.resolve()
    if source == output or source in output.parents:
        p.error('output must be outside source')
    output.mkdir(parents=True, exist_ok=False)
    series = ROOT/'patches/mainline-official'
    patches = [series/n for n in (series/'series').read_text().splitlines() if n]
    files = {line[6:] for patch in patches for line in patch.read_text().splitlines() if line.startswith('+++ b/')}
    new_files = set()
    for patch in patches:
        lines = patch.read_text().splitlines()
        new_files.update(lines[i+1][6:] for i, line in enumerate(lines[:-1]) if line == '--- /dev/null' and lines[i+1].startswith('+++ b/'))
    lock = json.loads((ROOT/'profiles/mainline-official/sources.json').read_text())
    revision = lock['pull_requests']['1800']['head']
    for name in files:
        if name in new_files:
            continue
        result = subprocess.run(['git', '-C', str(source), 'show', revision+':'+name], capture_output=True)
        if result.returncode == 0:
            target = output/name; target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(result.stdout)
        else:
            raise RuntimeError(result.stderr.decode())
    for patch in patches:
        subprocess.run(['patch', '--batch', '--fuzz=0', '-p1', '-i', str(patch)], cwd=output, check=True)
    hashes = {}
    for name in sorted(files):
        actual = (output/name).read_bytes()
        if actual != (source/name).read_bytes():
            raise ValueError('patch replay differs: '+name)
        hashes[name] = hashlib.sha256(actual).hexdigest()
    (output/'replay.json').write_text(json.dumps(dict(base=revision, files=hashes), indent=2)+'\n')
    print('PASS exact adaptation replay:',len(files),'files')


if __name__ == '__main__':
    main()
