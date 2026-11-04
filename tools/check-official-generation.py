#!/usr/bin/env python3
"""Exercise Cargo reuse/rebuild after a private SDK generation change.

Requires an owned scratch source checkout: appends/restores a comment in the
host qualification fixture, reinstalls its managed prefix, and always restores
both source and installation. No SDK headers, libraries or std are modified.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('source', 'install-command', 'acceptance', 'prefix', 'output'):
        p.add_argument('--'+name, type=Path, required=True)
    a = p.parse_args(); out = a.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    prefix = a.prefix.resolve(); acceptance = a.acceptance.resolve()
    installer = json.loads(a.install_command.read_text())
    cargo = json.loads((acceptance/'std-main-command.json').read_text()) + ['-v']
    fixture = a.source.resolve()/'tools/crosstools/rust/unmodified/consumer-host-probe/src/lib.rs'
    original = fixture.read_bytes()
    env = {k:v for k,v in os.environ.items() if not k.startswith('CARGO_') and k not in
           ('RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','RUSTC_BOOTSTRAP')}
    env.update(RUSTC=str(prefix/'bin/rustc'), CARGO_HOME=str(acceptance/'cargo-home'),
               CARGO_TARGET_DIR=str(acceptance/'target'), CARGO_INCREMENTAL='0')
    def run(command, name):
        with (out/(name+'.log')).open('w') as log:
            subprocess.run(command, env=env, cwd=acceptance, stdout=log, stderr=subprocess.STDOUT,
                           timeout=300, check=True)
        return (out/(name+'.log')).read_text()
    def generation():
        return json.loads((prefix/'aros-official-rust.json').read_text())['generation']
    first = generation()
    log = run(cargo, 'same-generation')
    if '--crate-name aros_acceptance' in log or 'Fresh aros-acceptance' not in log:
        raise RuntimeError('unchanged consumer did not reuse its artifacts')
    try:
        fixture.write_bytes(original+b'\n// Private SDK generation invalidation probe.\n')
        run(installer, 'install-next-generation')
        second = generation()
        if second == first:
            raise RuntimeError('generation did not change with its input')
        log = run(cargo, 'changed-generation')
        if '--crate-name aros_acceptance' not in log or second not in log:
            raise RuntimeError('Cargo did not rebuild the target with the new generation')
    finally:
        fixture.write_bytes(original)
        run(installer, 'restore-generation')
    if generation() != first:
        raise RuntimeError('original generation was not restored')
    run(cargo, 'restored-generation')
    (out/'result.json').write_text(json.dumps(dict(cached='PASS', invalidation='PASS', restored='PASS',
        initial=first, changed=second, runtime='UNTESTED'), indent=2)+'\n')
    print('PASS cached/rebuilt/restored SDK generations; no native execution')


if __name__ == '__main__':
    main()
