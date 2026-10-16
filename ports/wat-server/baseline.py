#!/usr/bin/env python3
"""Build an isolated x86_64 v11 WAT bundle, then verify stopped guest reports."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib
import uuid

REPO = Path(__file__).resolve().parents[2]
PORT = Path(__file__).resolve().parent
PINNED_RUST = 'rustc 1.100.0-nightly (0dfb098f3 2026-08-31)'


def check_results(results, prefix, profile, probes=True, payload=None):
    if profile not in ('abort', 'unwind') or not re.fullmatch(r'[A-Za-z0-9_-]{1,32}', prefix):
        raise ValueError('invalid profile or report prefix')
    checked = {}

    def read(suffix):
        path = Path(results) / (prefix + suffix)
        try:
            body = path.read_text()
        except (OSError, UnicodeError) as error:
            raise ValueError('missing/unreadable report ' + path.name) from error
        checked[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
        return body

    def case(name, rc, transcript, markers):
        if read(rc).strip() != '0':
            raise ValueError(name + ': nonzero return code')
        body = read(transcript)
        for forbidden in ('CASE_ERROR', 'REQUEST_TRACE_DROPPED', 'AROS_PROBE_GATE_TIMEOUT', 'write_error', 'panicked'):
            if forbidden in body:
                raise ValueError(name + ': ' + forbidden)
        for marker in markers:
            if marker not in body:
                raise ValueError(name + ': missing ' + marker)
        return body

    if payload is not None:
        for suffix in ('-payload.json', '-complete.json'):
            try:
                receipt = json.loads(read(suffix))
            except (ValueError, TypeError) as error:
                raise ValueError('missing/invalid guest payload receipt ' + suffix) from error
            if receipt != payload:
                raise ValueError('guest payload receipt does not match this build: ' + suffix)
    if read('-done.txt').strip() != 'DONE':
        raise ValueError('guest suite did not finish')
    common = ['SERVER_STATUS exit status: 0', 'PASS shutdown, EOF, child wait and stderr join']
    case('baseline', '-rc.txt', '-transcript.txt', common + ['PASS 16 outstanding mixed requests matched by ID', 'STDERR_BYTES 0'])
    cases = ['baseline']
    if profile == 'unwind':
        final = 'PASS interactive shutdown, EOF, child wait and both reader joins'
        for name, markers in [
            ('interactive', ['PASS recovery query sees final document and last function']),
            ('shutdown', ['PASS shutdown response while request 10 remains at checkpoint']),
            ('active-exit', []), ('active-eof', ['OBSERVE close stdin with request 10 still paused']),
        ]:
            body = case(name, '-' + name + '.rc', '-' + name + '.txt', [common[0], final] + markers)
            if name.startswith('active-'):
                events = [event for event, identity in re.findall(r'REQUEST_TRACE \d+ (\w+) Integer\((\d+)\)', body) if identity == '10']
                expected = ['registered', 'worker_start', 'salsa_checkpoint', 'teardown_cancel_applied',
                            'query_done', 'write_ok', 'token_removed', 'workers_joined']
                if events != expected:
                    raise ValueError(name + ': invalid lifecycle; expected workers_joined after token removal')
            cases.append(name)
        if probes:
            case('unwind', '-unwind.rc', '-unwind.txt', [
                'PASS resume_unwind payload and destructor', 'PASS catch and rethrow',
                'PASS 64 caught worker unwinds, destructors and joins',
                'PASS thread panic payload and destructor', 'DONE unwind acceptance'])
            case('cancellation', '-cancellation.rc', '-cancellation.txt', [
                'PASS uncancelled Salsa query', 'CANCELLATION_REQUESTED before query',
                'PASS cancelled Salsa query caught without terminating process'])
            cases += ['unwind', 'cancellation']
    return {'status': 'passed', 'profile': profile, 'cases': cases, 'report_sha256': checked}


def source_hash(root):
    digest = hashlib.sha256()
    for directory, dirs, files in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in ('target', '.git', '__pycache__'))
        for name in sorted(files):
            path = Path(directory) / name
            digest.update(path.relative_to(root).as_posix().encode() + b'\0' + hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def port_inputs():
    files = [p for p in PORT.iterdir() if p.is_file()]
    files += [REPO / name for name in ('x86_64-aros.json', 'aros-env.toml')]
    return {p.relative_to(REPO).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(files)}


def build(args):
    # Import only on the build path so checking reports never needs a compiler.
    from native_profile import native_options
    options = native_options(REPO, args.profile, os.environ)
    if not re.fullmatch(r'[A-Za-z0-9_-]{1,32}', args.prefix):
        raise ValueError('invalid guest report prefix')
    output = args.output.resolve()
    if output.is_relative_to(args.source.resolve()):
        raise ValueError('output must be outside the pinned upstream source')
    if output.exists():
        raise ValueError('output must be a new directory; builds never reuse another profile')
    rust = subprocess.check_output(['rustc', '+' + args.toolchain, '--version'], text=True).strip()
    if rust != PINNED_RUST:
        raise ValueError('expected ' + PINNED_RUST + '; got ' + rust)
    if not (REPO / 'aros-env.toml').is_file() or not (REPO / 'x86_64-aros.json').is_file():
        raise ValueError('run setup.sh for the selected v11 GCC/SDK first')
    sysroot = Path(subprocess.check_output(['rustc', '+' + args.toolchain, '--print', 'sysroot'], text=True).strip()).resolve()
    library = sysroot / 'lib/rustlib/src/rust/library'
    if not library.is_dir():
        raise ValueError('selected isolated toolchain is missing Rust library sources')
    inputs = port_inputs()
    rust_source = source_hash(library)
    output.mkdir(parents=True)
    (output / 'logs').mkdir()
    artifacts = output / 'artifacts'
    artifacts.mkdir()
    metadata = {'status': 'building', 'platform': 'x86_64-aros-v11', 'profile': args.profile,
                'rustc': rust, 'toolchain': args.toolchain, 'prefix': args.prefix,
                'resolved_sysroot': str(sysroot), 'rust_library_sha256': rust_source, 'input_sha256': inputs,
                'commands': [], 'sdk_tools': tomllib.loads((REPO / 'aros-env.toml').read_text())['env']}
    env = dict(os.environ, CARGO_HOME=str(args.cargo_home.resolve()))
    manifest = output / 'run.json'

    def save():
        manifest.write_text(json.dumps(metadata, indent=2) + '\n')

    def run(command, label, target=None):
        command = [str(x) for x in command]
        metadata['commands'].append({'command': command, 'log': 'logs/' + label + '.log'})
        save()
        child_env = dict(env)
        if target:
            child_env['CARGO_TARGET_DIR'] = str(target)
        with (output / 'logs' / (label + '.log')).open('w') as log:
            subprocess.run(command, cwd=output, env=child_env, stdout=log, stderr=subprocess.STDOUT, check=True)

    try:
        run(['rustc', '+' + args.toolchain, '--print', 'cfg', '--target', REPO / 'x86_64-aros.json',
             '-Zunstable-options', '-Cpanic=' + args.profile], 'profile-cfg')
        if 'panic="' + args.profile + '"' not in (output / 'logs/profile-cfg.log').read_text().splitlines():
            raise ValueError('compiler did not select requested panic profile')
        libc = output / 'libc-source'
        shutil.copytree(REPO / 'std-groundwork/libc-aros', libc,
                        ignore=shutil.ignore_patterns('target', '.git', '__pycache__', '.DS_Store'))
        metadata['libc_source_sha256'] = source_hash(libc)
        profile_args = ['--aros', '--panic-profile', args.profile, '--libc-source', libc]

        target = output / 'build' / args.profile / 'server'
        ordinary = output / 'ordinary'
        run([sys.executable, PORT / 'prepare.py', args.source.resolve(), ordinary], 'prepare-ordinary')
        run([sys.executable, PORT / 'build.py', ordinary, '--toolchain', args.toolchain, *profile_args], 'build-ordinary', target)
        server = artifacts / 'wat-server-ordinary'
        shutil.copy2(target / 'x86_64-aros/release/wat_server', server)
        diagnostic = None
        if args.profile == 'unwind':
            diagnostic = output / 'diagnostic'
            run([sys.executable, PORT / 'prepare.py', args.source.resolve(), diagnostic, '--probe-checkpoint'], 'prepare-diagnostic')
            run([sys.executable, PORT / 'build.py', diagnostic, '--toolchain', args.toolchain, *profile_args], 'build-diagnostic', target)
            server = artifacts / 'wat-server-diagnostic'
            shutil.copy2(target / 'x86_64-aros/release/wat_server', server)
            run([sys.executable, PORT / 'build.py', diagnostic, '--toolchain', args.toolchain, *profile_args, '--cancellation-probe'], 'build-cancellation', target)
            shutil.copy2(target / 'x86_64-aros/release/wat-cancellation-probe', artifacts / 'cancel-probe')
        # Preserve original Cargo.lock and bypass package-local cross configuration.
        client_root = output / 'client-source'
        for relative in ('transport', 'std-groundwork/wat-lsp-acceptance', 'std-groundwork/unwind-acceptance'):
            shutil.copytree(REPO / relative, client_root / relative,
                            ignore=shutil.ignore_patterns('target', '.cargo', '.DS_Store'))
        inventory = output / ((diagnostic or ordinary).name + '-deps/inventory-0.3.24')
        config = ['--config', REPO / 'aros-env.toml', '--config',
                  'patch.crates-io.libc.path=' + json.dumps(str(libc)),
                  '--config', 'patch.crates-io.inventory.path=' + json.dumps(str(inventory))]
        cargo = ['cargo', '+' + args.toolchain, 'build', '--release', '--offline']
        client_target = output / 'build' / args.profile / 'clients'
        run(cargo + ['--manifest-path', client_root / 'std-groundwork/wat-lsp-acceptance/Cargo.toml'] + options + config,
            'build-clients', client_target)
        binaries = client_target / 'x86_64-aros/release'
        stage = [sys.executable, PORT / 'stage.py', server, binaries / 'wat-lsp-acceptance', output / 'payload', '--prefix', args.prefix]
        if args.profile == 'unwind':
            run(cargo + ['--manifest-path', client_root / 'std-groundwork/unwind-acceptance/Cargo.toml'] + options + config,
                'build-unwind', client_target)
            stage += ['--interactive-client', binaries / 'wat-interactive-acceptance', '--probe-checkpoint',
                      '--unwind-probe', binaries / 'unwind-acceptance', '--cancellation-probe', artifacts / 'cancel-probe',
                      '--baseline-server', artifacts / 'wat-server-ordinary']
        run(stage, 'stage')
        script = output / 'payload/run-tests'
        marker = 'Echo START >RESULTS:' + args.prefix + '-start.txt'
        # Never overwrite a previous suite: FAT report overwrite is unreliable on the tested guest.
        suffixes = ['-start.txt', '-done.txt', '-payload.json', '-complete.json', '-rc.txt', '-transcript.txt']
        if args.profile == 'unwind':
            for name in ('interactive', 'shutdown', 'active-exit', 'active-eof', 'unwind', 'cancellation'):
                suffixes += ['-' + name + '.rc', '-' + name + '.txt']
        guard = 'FailAt 21\n'
        for suffix in suffixes:
            guard += ('If EXISTS RESULTS:' + args.prefix + suffix + '\n'
                      'Echo "Refusing existing baseline evidence"\nQuit 20\nEndIf\n')
        text = script.read_text().replace('FailAt 21\n', guard, 1)
        text = text.replace(marker, marker + '\nCopy RAM:wl/payload-sha256.json RESULTS:' + args.prefix + '-payload.json')
        done = 'Echo DONE >RESULTS:' + args.prefix + '-done.txt'
        text = text.replace(done, 'Copy RAM:wl/payload-sha256.json RESULTS:' + args.prefix + '-complete.json\n' + done)
        script.write_text(text)
        (output / 'payload/run-id').write_text(str(uuid.uuid4()) + '\n')
        metadata['payload_sha256'] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                      for p in (output / 'payload').iterdir() if p.is_file()}
        (output / 'payload/payload-sha256.json').write_text(json.dumps(metadata['payload_sha256'], indent=2) + '\n')
        if inputs != port_inputs() or rust_source != source_hash(library):
            raise ValueError('port inputs or selected Rust sources changed during build; use a fresh output')
        metadata['client_source_sha256'] = {p.relative_to(client_root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                                           for p in sorted(client_root.rglob('*')) if p.is_file()}
        metadata['status'] = 'built-not-run'
        save()
        print(manifest)
    except Exception as error:
        metadata.update(status='failed', error=str(error))
        save()
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    make = commands.add_parser('build')
    make.add_argument('source', type=Path)
    make.add_argument('output', type=Path)
    make.add_argument('--toolchain', required=True)
    make.add_argument('--cargo-home', type=Path, required=True)
    make.add_argument('--profile', choices=('abort', 'unwind'), default='abort')
    make.add_argument('--prefix', required=True)
    check = commands.add_parser('check')
    check.add_argument('run', type=Path)
    check.add_argument('results', type=Path)
    check.add_argument('--machine', required=True)
    check.add_argument('--runtime-profile', required=True)
    args = parser.parse_args()
    try:
        if args.command == 'build':
            build(args)
        else:
            metadata = json.loads((args.run / 'run.json').read_text())
            if metadata['status'] != 'built-not-run':
                raise ValueError('run is not a completed build')
            report = check_results(args.results, metadata['prefix'], metadata['profile'], payload=metadata['payload_sha256'])
            report.update(machine=args.machine, runtime_profile=args.runtime_profile,
                          build_run_sha256=hashlib.sha256((args.run / 'run.json').read_bytes()).hexdigest())
            (args.run / 'validation.json').write_text(json.dumps(report, indent=2) + '\n')
            print(json.dumps(report, indent=2))
    except (ValueError, OSError, subprocess.SubprocessError, KeyError) as error:
        if args.command == 'check' and args.run.is_dir():
            (args.run / 'validation.json').write_text(json.dumps({'status': 'failed', 'error': str(error)}, indent=2) + '\n')
        parser.exit(1, str(error) + '\n')


if __name__ == '__main__':
    main()
