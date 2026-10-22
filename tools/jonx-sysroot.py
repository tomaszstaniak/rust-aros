#!/usr/bin/env python3
"""Copy pinned Jonx rust-src into a new private sysroot, without rustup mutation."""
import argparse
from pathlib import Path
import shlex
import shutil
import subprocess

REVISION = '2e2845efb6aaa111b5a26c9685e96d08fad340f4'
BASE = 'nightly-2026-06-27'
VERSION = 'rustc 1.98.0-nightly (ce9954c0c 2026-06-26)'


def validate_source(source, revision=REVISION):
    actual = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if actual != revision:
        raise ValueError(f'wrong Jonx source revision: {actual}; require {revision}')
    dirty = subprocess.check_output(['git', '-C', str(source), 'status', '--porcelain'], text=True)
    if dirty:
        raise ValueError('Jonx source must be a clean checkout (including untracked files)')


def create_sysroot(base, source, dest):
    # Refuse dangling links as well as existing directories. mkdir owns the
    # destination atomically; cleanup is limited to that newly created path.
    dest.mkdir()
    try:
        (dest / 'bin').mkdir()
        (dest / 'lib/rustlib/src').mkdir(parents=True)
        for binary in (base / 'bin').iterdir():
            args = [str(binary)]
            if binary.name == 'rustc':
                args += ['--sysroot', str(dest)]
            wrapper = dest / 'bin' / binary.name
            wrapper.write_text('#!/bin/sh\nexec ' + shlex.join(args) + ' "$@"\n')
            wrapper.chmod(0o755)
        for entry in (base / 'lib/rustlib').iterdir():
            if entry.name != 'src':
                (dest / 'lib/rustlib' / entry.name).symlink_to(entry)
        for entry in (base / 'lib').iterdir():
            if entry.is_file():
                (dest / 'lib' / entry.name).symlink_to(entry)
        sources = dest / 'lib/rustlib/src/rust'
        sources.mkdir()
        for name in ('library', 'src'):
            shutil.copytree(source / name, sources / name, symlinks=True)
    except BaseException:
        shutil.rmtree(dest)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('new_sysroot', type=Path)
    args = parser.parse_args()
    source, dest = args.source.resolve(), args.new_sysroot.absolute()
    validate_source(source)
    version = subprocess.check_output(['rustup', 'run', BASE, 'rustc', '-V'], text=True).strip()
    if version != VERSION:
        raise ValueError(f'wrong compiler: {version}; require {VERSION}')
    base = Path(subprocess.check_output(['rustup', 'run', BASE, 'rustc', '--print', 'sysroot'], text=True).strip())
    create_sysroot(base, source, dest)
    print(f'{dest}: {version}; Jonx {REVISION}')


if __name__ == '__main__':
    main()
