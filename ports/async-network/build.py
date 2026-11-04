#!/usr/bin/env python3
"""Build the checked async consumer in a new private output, never a shared sysroot."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
from prepare import PORT, REPO, prepare, sha
sys.path.insert(0,str(PORT.parent/'wat-server'))
from native_profile import native_options

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archives',type=Path)
    parser.add_argument('output',type=Path)
    parser.add_argument('--toolchain',required=True)
    parser.add_argument('--cargo-home',type=Path,required=True)
    parser.add_argument('--profile',choices=('abort','unwind'),default='abort')
    args=parser.parse_args()
    try:
        options=native_options(REPO,args.profile,os.environ)
        rust=subprocess.check_output(['rustc','+'+args.toolchain,'--version'],text=True).strip()
        if rust != 'rustc 1.100.0-nightly (0dfb098f3 2026-08-31)':
            raise ValueError('select the pinned compiler; got '+rust)
        status=prepare(args.archives,args.output)
        output=args.output.resolve()
        environment=dict(os.environ,CARGO_HOME=str(args.cargo_home.resolve()),CARGO_TARGET_DIR=str(output/'target'))
        command=['cargo','+'+args.toolchain,'build','--manifest-path',str(output/'consumer/Cargo.toml'),'--release','--offline','--locked','--config',str(REPO/'aros-env.toml'),*options]
        for name, directory in [('libc','libc-source'),('polling','polling'),('async-io','async-io'),('async-net','async-net')]:
            command+=['--config','patch.crates-io.'+name+'.path='+json.dumps(str(output/directory))]
        metadata={'status':'building','compiler':rust,'profile':args.profile,'command':command,'prepare_sha256':sha(output/'prepare.json'),'sysroot':subprocess.check_output(['rustc','+'+args.toolchain,'--print','sysroot'],text=True).strip()}
        manifest=output/'build.json';manifest.write_text(json.dumps(metadata,indent=2)+'\n')
        with (output/'logs/build.log').open('w') as log:
            result=subprocess.run(command,env=environment,cwd=output,stdout=log,stderr=subprocess.STDOUT)
        metadata['returncode']=result.returncode
        if result.returncode:
            metadata['status']='failed';manifest.write_text(json.dumps(metadata,indent=2)+'\n')
            raise ValueError('build failed; see '+str(output/'logs/build.log'))
        binary=output/'target/x86_64-aros/release/aros-async-consumer'
        metadata.update(status='built-not-run',binary_sha256=sha(binary),lock_sha256=sha(output/'consumer/Cargo.lock'))
        manifest.write_text(json.dumps(metadata,indent=2)+'\n')
        print(json.dumps(metadata,indent=2))
    except (OSError,ValueError,subprocess.SubprocessError) as error:
        parser.exit(1,str(error)+'\n')

if __name__=='__main__':
    main()
