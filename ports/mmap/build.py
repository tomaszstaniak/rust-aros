#!/usr/bin/env python3
"""Build a private pinned font mapping diagnostic for x86_64 ABIv11."""
import argparse,hashlib,json,os,re,shutil,subprocess,sys,uuid
from pathlib import Path
REPO=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(REPO/'ports/wat-server'))
from native_profile import native_options

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def build(args):
    out=args.output.resolve()
    if out.exists() or out.is_relative_to(REPO):raise ValueError('select a new output outside repository')
    rust=subprocess.check_output(['rustc','+'+args.toolchain,'--version'],text=True).strip()
    if rust!='rustc 1.100.0-nightly (0dfb098f3 2026-08-31)':raise ValueError('select pinned compiler')
    options=native_options(REPO,args.profile,os.environ)
    out.mkdir(parents=True)
    for name,source in [('consumer','std-groundwork/mmap-acceptance'),('libc-source','std-groundwork/libc-aros')]:
        shutil.copytree(REPO/source,out/name,ignore=shutil.ignore_patterns('target','.cargo','__pycache__','.DS_Store'))
    stage=out/'payload';stage.mkdir()
    shutil.copy2(args.font,stage/'font.ttf');shutil.copy2(args.font_license,stage/'FONT-LICENSE.txt')
    run=str(uuid.uuid4());prefix='mm'+run.replace('-','')[:14]
    command=['cargo','+'+args.toolchain,'build','--release','--offline','--locked','--manifest-path',str(out/'consumer/Cargo.toml'),'--config',str(REPO/'aros-env.toml'),*options,'--config','patch.crates-io.libc.path='+json.dumps(str(out/'libc-source'))]
    metadata={'status':'building','run_id':run,'prefix':prefix,'profile':args.profile,'compiler':rust,'command':command,'font_sha256':sha(args.font),'license_sha256':sha(args.font_license),'source_sha256':{p.relative_to(out).as_posix():sha(p) for name in ['consumer','libc-source'] for p in sorted((out/name).rglob('*')) if p.is_file()}}
    def save():(out/'build.json').write_text(json.dumps(metadata,indent=2)+'\n')
    save()
    env=dict(os.environ,CARGO_HOME=str(args.cargo_home.resolve()),CARGO_TARGET_DIR=str(out/'target'),AROS_MMAP_RUN_ID=run)
    with (out/'build.log').open('w') as log:result=subprocess.run(command,env=env,cwd=out,stdout=log,stderr=subprocess.STDOUT)
    metadata['returncode']=result.returncode
    if result.returncode:metadata['status']='failed';save();raise ValueError('build failed; see '+str(out/'build.log'))
    shutil.copy2(out/'target/x86_64-aros/release/aros-mmap-acceptance',stage/'mmap-check')
    lines=['FailAt 21','Stack 1048576']
    for suffix in ['.log','.rc']:
        lines += [f'If EXISTS RESULTS:{prefix}{suffix}','Echo "Refusing existing mmap evidence"','Quit 20','EndIf']
    lines += [f'RAM:mm/mmap-check RAM:mm/font.ttf >RESULTS:{prefix}.log',f'Echo $RC >RESULTS:{prefix}.rc']
    (stage/'run-tests').write_text('\n'.join(lines)+'\n')
    metadata.update(status='built-not-run',payload_sha256={p.name:sha(p) for p in stage.iterdir() if p.is_file()});save();return metadata

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('output',type=Path);parser.add_argument('--font',type=Path,required=True);parser.add_argument('--font-license',type=Path,required=True);parser.add_argument('--toolchain',required=True);parser.add_argument('--cargo-home',type=Path,required=True);parser.add_argument('--profile',choices=['abort','unwind'],default='unwind');args=parser.parse_args()
    try:print(json.dumps(build(args),indent=2))
    except (OSError,ValueError,subprocess.SubprocessError) as error:parser.exit(1,str(error)+'\n')
if __name__=='__main__':main()
