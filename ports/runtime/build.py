#!/usr/bin/env python3
"""Build private before/after v11 probes without modifying the SDK."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import uuid

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'ports/wat-server'))
from native_profile import native_options
PTHREAD_COMMIT = '5376f09bc1ed0c36a12fbb096406013f8a046e89'
CRATES = {'parking_lot_core-0.9.12': '2621685985a2ebf1c516881c026032ac7deafcda1a2c9b7850dc81e3dfcb64c1',
          'rayon-core-1.13.0': '22e18b0f0062d30d4230b2e85ff77fdfe4326feb054b9783a3460d8435c8ab91'}
FLAGS = ['-O2', '-mcmodel=large', '-fno-omit-frame-pointer']


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(args, *, cwd=None, env=None, log=None):
    if log:
        with log.open('w') as stream:
            subprocess.run(list(map(str, args)), cwd=cwd, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
        return b''
    return subprocess.check_output(list(map(str, args)), cwd=cwd, env=env, stderr=subprocess.STDOUT)


def objkey(gcc, path):
    output = command([gcc.replace('-gcc', '-objdump'), '-d', '-r', '--no-show-raw-insn', path])
    return hashlib.sha256(output.split(b'\n', 4)[-1]).hexdigest()


def build(args):
    out = args.output.resolve()
    if out.exists() or out.is_relative_to(REPO):
        raise ValueError('output must be new and outside repository')
    config = json.loads(args.config.read_text())
    compiler = command(['rustc', '+' + config['toolchain'], '--version']).decode().strip()
    if compiler != 'rustc 1.100.0-nightly (0dfb098f3 2026-08-31)':
        raise ValueError('wrong pinned Rust compiler')
    options = native_options(REPO, 'unwind', os.environ)
    gcc, sdk, ar = [config[k] for k in ('gcc', 'sdk', 'ar')]
    out.mkdir(parents=True)
    metadata = {'status': 'building', 'run_id': str(uuid.uuid4()), 'profile': 'unwind', 'compiler': compiler,
                'configuration': config, 'sdk_pthread_sha256': sha(Path(sdk) / 'lib/libpthread.a'),
                'aros_pthread_commit': PTHREAD_COMMIT, 'cflags': FLAGS, 'variants': {}}
    def save():
        (out / 'build.json').write_text(json.dumps(metadata, indent=2) + '\n')
    save()
    archive = command(['git', '-C', config['aros_src'], 'archive', PTHREAD_COMMIT, 'compiler/pthread'])
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(out / 'source', filter='data')
    source = out / 'source/compiler/pthread'
    names, collecting = [], False
    for line in (source / 'mmakefile.src').read_text().splitlines():
        if line.startswith(('LINKLIBFILES', 'PTHREADFUNCS')):
            collecting = True; line = line.split(':=', 1)[1]
        if collecting:
            names += line.replace('\\', ' ').split()
            collecting = line.rstrip().endswith('\\')
    sdk_objects = out / 'sdk-objects'; sdk_objects.mkdir()
    command([ar, 'x', Path(sdk) / 'lib/libpthread.a'], cwd=sdk_objects)
    if sorted(p.stem for p in sdk_objects.glob('*.o')) != sorted(names):
        raise ValueError('SDK archive member mismatch')
    plain = out / 'pthread-before'; plain.mkdir()
    for name in names:
        command([gcc, '--sysroot=' + sdk, *FLAGS, '-I', source, '-c', source / (name + '.c'), '-o', plain / (name + '.o')])
    bad = [n for n in names if objkey(gcc, plain / (n + '.o')) != objkey(gcc, sdk_objects / (n + '.o'))]
    if bad:
        raise ValueError('SDK instructions/relocations differ: ' + repr(bad))
    metadata['sdk_object_matches'] = len(names); save()
    # Copy only versioned probe/runtime inputs. Source and archives are read-only.
    shutil.copytree(REPO / 'std-groundwork/libc-aros', out / 'libc-source', ignore=shutil.ignore_patterns('target', '.cargo', '.DS_Store'))
    shutil.copytree(REPO / 'std-groundwork/runtime-regressions', out / 'consumer', ignore=shutil.ignore_patterns('target', '.cargo', '.DS_Store'))
    shutil.copytree(REPO / 'ports/runtime', out / 'recipe', ignore=shutil.ignore_patterns('__pycache__'))
    for name, digest in CRATES.items():
        matches = list(Path(config['cargo_home']).glob('registry/cache/*/' + name + '.crate'))
        if len(matches) != 1 or sha(matches[0]) != digest:
            raise ValueError('missing or changed crate archive ' + name)
        for variant in ('before', 'after'):
            with tarfile.open(matches[0]) as tar:
                tar.extractall(out / variant, filter='data')
    # The independent crate comparisons both use fixed pthread objects.
    fixed = out / 'pthread-after'; fixed.mkdir()
    command(['patch', '--batch', '--fuzz=0', '-p1', '-i', out / 'recipe/patches/pthread.patch'], cwd=out / 'source')
    for name in ('pthread', 'pthread_create'):
        command([gcc, '--sysroot=' + sdk, *FLAGS, '-I', source, '-c', source / (name + '.c'), '-o', fixed / (name + '.o')])
    payload = out / 'payload'; payload.mkdir()
    command([gcc, '--sysroot=' + sdk, *FLAGS, out / 'recipe/receipt.c', '-o', payload / 'receipt'])
    command([gcc, '--sysroot=' + sdk, *FLAGS, '-c', out / 'recipe/deadline.c', '-o', out / 'deadline.o'])
    for variant in ('before', 'after'):
        identity = metadata['run_id'] + '-' + variant
        v = {'identity': identity, 'commands': [], 'reports': []}; metadata['variants'][variant] = v
        variant_dir = out / variant
        if variant == 'after':
            for crate, patch in [('parking_lot_core-0.9.12', 'parking.patch'), ('rayon-core-1.13.0', 'rayon.patch')]:
                command(['patch', '--batch', '--fuzz=0', '-p1', '-i', out / 'recipe/patches' / patch], cwd=variant_dir / crate)
        env = dict(os.environ, CARGO_HOME=config['cargo_home'], CARGO_TARGET_DIR=str(out / ('target-' + variant)), RUNTIME_RUN_ID=identity)
        patches = []
        for key, path in [('libc', out / 'libc-source'), ('parking_lot_core', variant_dir / 'parking_lot_core-0.9.12'), ('rayon-core', variant_dir / 'rayon-core-1.13.0')]:
            patches += ['--config', 'patch.crates-io.' + key + '.path=' + json.dumps(str(path))]
        manifest = ['--manifest-path', str(out / 'consumer/Cargo.toml')]
        if not (out / 'consumer/Cargo.lock').exists():
            command(['cargo', '+' + config['toolchain'], 'generate-lockfile', '--offline', *manifest, *patches], env=env, cwd=out, log=out/'lock.log')
        objects = [(plain if variant == 'before' else fixed) / (n + '.o') for n in ('pthread', 'pthread_create')]
        ccmd = [gcc, '--sysroot=' + sdk, *FLAGS, '-I', source, '-DRUNTIME_RUN_ID=' + json.dumps(identity), out / 'recipe/detach.c', *objects, '-lpthread', '-o', payload / ('detach-' + variant)]
        command(ccmd, log=out / ('detach-' + variant + '.log')); v['commands'].append(list(map(str, ccmd)))
        for case in ('parking', 'rayon'):
            args_build = ['cargo', '+' + config['toolchain'], 'build', '--release', '--offline', '--locked', '--bin', case, *manifest, '--config', str(REPO/'aros-env.toml'), *options, *patches]
            prelink = ' '.join(str(fixed / (n + '.o')) for n in ('pthread', 'pthread_create'))
            if case == 'parking':
                prelink += ' ' + str(out / 'deadline.o')
            args_build += ['--config', 'build.rustflags=' + json.dumps(['-Cpanic=unwind', '-Cforce-unwind-tables=yes', '-Zpre-link-args=' + prelink] + (['-Clink-arg=-Wl,--wrap=pthread_cond_timedwait'] if case == 'parking' else []))]
            if variant == 'after' and case == 'rayon':
                args_build += ['--features', 'shutdown']
            v['commands'].append(args_build); save()
            command(args_build, cwd=out, env=env, log=out / (case + '-' + variant + '.log'))
            shutil.copyfile(out / ('target-' + variant) / 'x86_64-aros/release' / case, payload / (case + '-' + variant))
        for case in ('detach', 'parking', 'rayon'):
            prefix = 'rt' + metadata['run_id'].replace('-', '')[:10] + '-' + variant + '-' + case
            v['reports'] += [prefix + '.log', prefix + '.rc']
            lines = ['FailAt 21', 'Stack 1048576']
            for suffix in ('.log', '.rc'):
                lines += ['If EXISTS RESULTS:' + prefix + suffix, 'Echo "Refusing existing runtime evidence"', 'Quit 20', 'EndIf']
            lines += [f'RAM:rt/receipt {identity} {case} start 0', 'If ERROR', 'Quit 20', 'EndIf', f'RAM:rt/{case}-{variant} >RESULTS:{prefix}.log', f'Echo $RC >RESULTS:{prefix}.rc', f'RAM:rt/receipt {identity} {case} end RESULTS:{prefix}.rc']
            (payload / ('run-' + variant + '-' + case)).write_text('\n'.join(lines) + '\n')
        (payload / ('run-' + variant)).write_text('FailAt 21\n' + ''.join(f'Run Execute RAM:rt/run-{variant}-{case}\n' for case in ('detach','parking','rayon')))
    metadata.update(status='built-not-run', payload_sha256={p.name: sha(p) for p in payload.iterdir()},
                    input_sha256={p.relative_to(out).as_posix(): sha(p) for directory in ('recipe','consumer','libc-source','source','before','after','pthread-before','pthread-after') for p in sorted((out/directory).rglob('*')) if p.is_file()})
    save()
    print(json.dumps({'status': metadata['status'], 'run_id': metadata['run_id'], 'sdk_object_matches': len(names)}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('config', type=Path); parser.add_argument('output', type=Path)
    args = parser.parse_args()
    try:
        build(args)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, str(error) + '\n')
if __name__ == '__main__':
    main()
