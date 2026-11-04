#!/usr/bin/env python3
"""Generate private PR #1799 POSIX imports; never modify the input SDK."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('source', 'sdk', 'cc', 'ar', 'genmodule', 'output'):
        p.add_argument('--' + name, type=Path, required=True)
    p.add_argument('--cc-arg', action='append', default=[])
    a = p.parse_args()
    output = a.output.resolve()
    if any(output == root or root in output.parents for root in (a.source.resolve(), a.sdk.resolve())):
        p.error('output must be outside source and SDK inputs')
    config = a.source / 'compiler/crt/posixc/posixc.conf'
    frozen = {str(x.resolve()): digest(x) for x in (config, a.cc, a.ar, a.genmodule)}
    a.output.mkdir(parents=True, exist_ok=False)
    module = a.output / 'module'
    for directory in ('proto', 'clib', 'inline', 'defines'):
        (module / 'include' / directory).mkdir(parents=True)
    commands = []
    def run(command):
        commands.append([str(x) for x in command])
        result = subprocess.run(commands[-1], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        with (a.output / 'commands.log').open('a') as log:
            log.write(result.stdout)
        result.check_returncode()
    for action, destination in (('writefiles', module), ('writelibdefs', module / 'include'),
                                ('writeincludes', module / 'include')):
        run([a.genmodule, '-c', config, '-d', destination, action, 'posixc', 'library'])
    objects = []
    # Generate all four imports: older SDK generations may omit the read exports too.
    for name in ('pread', 'pread64', 'pwrite', 'pwrite64'):
        obj = a.output / (name + '.o')
        run([a.cc, *a.cc_arg, '--sysroot=' + str(a.sdk.resolve()), '-fno-pic', '-mcmodel=large',
             '-I' + str(module / 'include'), '-c', module / ('posixc_' + name + '_stub.c'), '-o', obj])
        objects.append(obj)
    archive = a.output / 'libaros_positional.a'
    run([a.ar, 'rcs', archive, *objects])
    if frozen != {name: digest(Path(name)) for name in frozen}:
        raise RuntimeError('import generation inputs changed')
    (a.output / 'result.json').write_text(json.dumps(dict(inputs=frozen, commands=commands,
        archive=str(archive.resolve()), sha256=digest(archive), runtime='UNTESTED'), indent=2) + '\n')


if __name__ == '__main__':
    main()
