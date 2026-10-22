"""Versioned source identity for the private shared AROS sysroot."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMPILER = 'rustc 1.100.0-nightly (0dfb098f3 2026-08-31)'
PAL = '2e2845efb6aaa111b5a26c9685e96d08fad340f4'
PROFILES = ['aarch64-v1-raspi', 'x86_64-v11']
SOURCE = 'lib/rustlib/src/rust'


def digest_tree(root):
    if not root.is_dir():
        raise ValueError(f'missing source directory: {root}')
    digest = hashlib.sha256()
    for path in sorted(root.rglob('*')):
        if path.is_symlink():
            raise ValueError(f'source symlink is outside the contract: {path}')
        if path.is_file():
            digest.update(path.relative_to(root).as_posix().encode() + b'\0')
            digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def inputs(root=ROOT):
    files = [root / name for name in (
        'tools/common-sysroot.py', 'tools/jonx-sysroot.py', 'tools/sysroot_contract.py',
        'profiles/aarch64-v1-raspi.json', 'x86_64-aros.json.in')]
    for name in ('patches/jonx', 'std-groundwork/std-patch', 'std-groundwork/libc-aros'):
        files += [p for p in (root / name).rglob('*') if p.is_file()
                  and not any(part in ('target', '__pycache__') for part in p.parts)]
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(files)}


def identity(root=ROOT):
    return dict(schema=1, compiler=COMPILER, jonx=PAL, backends=PROFILES,
                native_sockets='disabled', adaptations=inputs(root))


def record(sysroot, root=ROOT):
    manifest = identity(root)
    manifest['sources'] = {name: digest_tree(sysroot / SOURCE / name)
                           for name in ('library', 'src')}
    (sysroot / 'aros-common.json').write_text(json.dumps(manifest, indent=2) + '\n')


def validate(sysroot, profile, root=ROOT):
    try:
        manifest = json.loads((sysroot / 'aros-common.json').read_text())
        expected = identity(root)
        if not isinstance(manifest, dict) or set(manifest) != set(expected) | {'sources'}:
            raise ValueError('invalid manifest schema')
        for key, value in expected.items():
            if type(manifest[key]) is not type(value) or manifest[key] != value:
                raise ValueError(f'sysroot {key} is stale or incompatible')
        if profile not in manifest['backends']:
            raise ValueError('unsupported profile')
        sources = manifest['sources']
        if not isinstance(sources, dict) or set(sources) != {'library', 'src'}:
            raise ValueError('invalid source inventory')
        for name, expected_hash in sources.items():
            if digest_tree(sysroot / SOURCE / name) != expected_hash:
                raise ValueError(f'modified sysroot sources: {name}')
        return manifest
    except (OSError, json.JSONDecodeError, TypeError, KeyError) as error:
        raise ValueError(f'invalid common sysroot: {error}') from error
