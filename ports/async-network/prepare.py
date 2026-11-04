#!/usr/bin/env python3
"""Prepare a private pinned async consumer; clean archives and shared inputs stay readonly."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile

PORT = Path(__file__).resolve().parent
REPO = PORT.parents[1]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def extract_checked(archive, destination, root_name, expected):
    archive, destination = Path(archive), Path(destination)
    if destination.exists() or destination.is_symlink():
        raise ValueError('destination already exists: ' + str(destination))
    if sha(archive) != expected:
        raise ValueError('archive checksum mismatch: ' + str(archive))
    with tarfile.open(archive, 'r:gz') as tar:
        members, seen = [], set()
        for member in tar.getmembers():
            path = PurePosixPath(member.name)
            if path.is_absolute() or '..' in path.parts or not path.parts or path.parts[0] != root_name:
                raise ValueError('unsafe archive path: ' + member.name)
            if not (member.isdir() or member.isfile()):
                raise ValueError('archive links/special files are unsupported: ' + member.name)
            relative = Path(*path.parts[1:])
            key = relative.as_posix().casefold()
            if key in seen:
                raise ValueError('duplicate archive member: ' + member.name)
            seen.add(key)
            members.append((member, relative))
        destination.mkdir(parents=True)
        try:
            for member, relative in members:
                target = destination / relative
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with tar.extractfile(member) as source, target.open('xb') as output:
                        shutil.copyfileobj(source, output)
                    target.chmod(0o755 if member.mode & 0o111 else 0o644)
        except BaseException:
            shutil.rmtree(destination)
            raise

def prepare(archives, output):
    archives, output = Path(archives).resolve(), Path(output).resolve()
    if output.exists() or output.is_symlink():
        raise ValueError('output must be a new directory')
    if output.is_relative_to(archives) or output.is_relative_to(REPO):
        raise ValueError('output must be outside archives and repository')
    pins = json.loads((PORT/'inputs.json').read_text())
    # Verify every input before creating any output.
    for name, pin in pins['crates'].items():
        archive = archives/(name+'-'+pin['version']+'.crate')
        if sha(archive) != pin['sha256']:
            raise ValueError('archive checksum mismatch: ' + str(archive))
    output.mkdir(parents=True)
    (output/'logs').mkdir()
    status = {'status': 'preparing', 'inputs': pins, 'port_sha256': {p.name:sha(p) for p in sorted(PORT.iterdir()) if p.is_file()}}
    manifest = output/'prepare.json'
    manifest.write_text(json.dumps(status, indent=2)+'\n')
    for name, pin in pins['crates'].items():
        destination = output/name
        extract_checked(archives/(name+'-'+pin['version']+'.crate'), destination, name+'-'+pin['version'], pin['sha256'])
        with (output/'logs'/('patch-'+name+'.log')).open('w') as log:
            subprocess.run(['patch','-p1','--batch','--forward','-i',str(PORT/(name+'-aros.patch'))], cwd=destination, stdout=log, stderr=subprocess.STDOUT, check=True)
    for source, destination in [('polling-aros.rs','polling/src/aros.rs'),('driver-aros.rs','async-io/src/driver_aros.rs'),('asyncio-aros.rs','async-io/src/aros.rs')]:
        shutil.copy2(PORT/source, output/destination)
    ignored = shutil.ignore_patterns('target', '.DS_Store', '__pycache__', '.cargo')
    shutil.copytree(REPO/'std-groundwork/libc-aros',output/'libc-source',ignore=ignored)
    shutil.copytree(REPO/'std-groundwork/async-consumer',output/'consumer',ignore=ignored)
    status['source_sha256']={p.relative_to(output).as_posix():sha(p) for p in sorted(output.rglob('*')) if p.is_file() and 'logs' not in p.relative_to(output).parts and p != manifest}
    status['status']='prepared-not-built'
    manifest.write_text(json.dumps(status,indent=2)+'\n')
    return status

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archives',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    try:
        status=prepare(args.archives,args.output)
        print(status['status'])
    except (OSError,ValueError,subprocess.SubprocessError,tarfile.TarError) as error:
        parser.exit(1,str(error)+'\n')

if __name__=='__main__':
    main()
