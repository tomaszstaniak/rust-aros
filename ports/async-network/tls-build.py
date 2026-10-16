#!/usr/bin/env python3
"""Private pinned apkg TLS build and Rust HTTPS consumer; no shared SDK changes."""
import argparse, datetime, io, json, os, shutil, subprocess, sys, tarfile, tomllib, uuid
from pathlib import Path, PurePosixPath
from prepare import PORT, REPO, prepare, sha
sys.path.insert(0,str(PORT.parent/'wat-server'))
from native_profile import native_options
APKG_REV='91ac4d8f00bbaf5ac5b057322d3937a61acd1cd7'
def build(args):
    options=native_options(REPO,args.profile,os.environ)
    rust=subprocess.check_output(['rustc','+'+args.toolchain,'--version'],text=True).strip()
    if rust!='rustc 1.100.0-nightly (0dfb098f3 2026-08-31)': raise ValueError('select pinned compiler')
    # Resolve the immutable Git tree, never copy a mutable apkg working tree.
    paths=['third_party/mbedtls','third_party/jitterentropy','src/tls','sdk/mbedtls-sources.txt','sdk/make-packages.sh','LICENSE']
    archive=subprocess.check_output(['git','-C',str(args.apkg),'archive',APKG_REV,*paths])
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        members=tar.getmembers()
        for m in members:
            p=PurePosixPath(m.name)
            if p.is_absolute() or '..' in p.parts or not (m.isdir() or m.isfile()): raise ValueError('unsafe apkg archive member '+m.name)
        prepare(args.archives,args.output)
        out=args.output.resolve(); source=out/'tls-source'; source.mkdir()
        for m in members:
            target=source/m.name
            if m.isdir(): target.mkdir(parents=True,exist_ok=True)
            else:
                target.parent.mkdir(parents=True,exist_ok=True)
                with tar.extractfile(m) as src, target.open('xb') as dst: shutil.copyfileobj(src,dst)
    shutil.rmtree(out/'consumer')
    shutil.copytree(REPO/'std-groundwork/async-https',out/'consumer',ignore=shutil.ignore_patterns('target','.DS_Store'))
    shutil.copy2(PORT/'tls_adapter.c',out/'tls_adapter.c'); shutil.copy2(PORT/'tls_adapter.h',out/'tls_adapter.h')
    settings=tomllib.loads((REPO/'aros-env.toml').read_text())['env']; cc=settings['AROS_GCC']; ar=settings['AROS_AR']; sdk=Path(settings['AROS_SDK'])
    inc=out/'include'; lib=out/'lib'; obj=out/'obj'; inc.mkdir(); lib.mkdir(); obj.mkdir()
    mb=source/'third_party/mbedtls'; je=source/'third_party/jitterentropy'
    shutil.copytree(mb/'include/mbedtls',inc/'mbedtls'); shutil.copytree(mb/'include/psa',inc/'psa')
    shutil.copy2(source/'src/tls/mbedtls_config.h',inc/'mbedtls_aros_config.h')
    shutil.copy2(je/'jitterentropy.h',inc/'jitterentropy.h'); shutil.copy2(source/'src/tls/jent-aros/jitterentropy-base-user.h',inc/'jitterentropy-base-user.h')
    base=['-std=gnu99','-I'+str(inc),'-I'+str(sdk/'include')]
    cfg=['-DMBEDTLS_CONFIG_FILE=<mbedtls_aros_config.h>','-DMBEDTLS_TEST_SW_INET_PTON','-I'+str(mb/'library')]
    metadata={'status':'building','run_id':str(uuid.uuid4()),'apkg_revision':APKG_REV,'compiler':rust,'sysroot':subprocess.check_output(['rustc','+'+args.toolchain,'--print','sysroot'],text=True).strip(),'profile':args.profile,'cc':subprocess.check_output([cc,'--version'],text=True).splitlines()[0],'sdk_crt_sha256':sha(sdk/'lib/libcrt.a'),'sources':{p.relative_to(out).as_posix():sha(p) for root in [source,*(out/name for name in ['consumer','polling','async-io','async-net','libc-source','include'])] for p in sorted(root.rglob('*')) if p.is_file()},'adapter':{name:sha(out/name) for name in ['tls_adapter.c','tls_adapter.h']},'commands':[],'artifacts':{}}
    manifest=out/'tls-build.json'
    def save(): manifest.write_text(json.dumps(metadata,indent=2)+'\n')
    save()
    with (out/'logs/tls-build.log').open('w') as log:
        def run(cmd,**kw):
            metadata['commands'].append(cmd); save(); subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True,**kw)
        def group(name,files,flags):
            directory=obj/name; directory.mkdir(); objects=[]
            for p in files:
                target=directory/(p.stem+'.o'); run([cc,*base,*flags,'-c',str(p),'-o',str(target)]); objects.append(str(target))
            run([ar,'rcs',str(lib/('lib'+name+'.a')),*objects])
        try:
            for line in (source/'sdk/mbedtls-sources.txt').read_text().splitlines():
                key,*files=line.split(); group('mbed'+key.removeprefix('src_'),[mb/'library'/f for f in files],['-O2',*cfg])
            group('mbedtls_aros',[source/'src/tls/platform_aros.c'],['-O2',*cfg])
            group('jitterentropy',sorted((je/'src').glob('*.c')),['-O0','-fwrapv','-I'+str(je/'src')])
            group('jitterentropy_mbedtls',[source/'src/tls/entropy_jent.c'],['-O2'])
            group('jitterentropy_mbedtls_fail',[source/'src/tls/entropy_jent.c'],['-O2','-DAROS_JENT_FORCE_FAIL'])
            group('rust_aros_tls',[out/'tls_adapter.c'],['-O2',*cfg])
            command=['cargo','+'+args.toolchain,'build','--manifest-path',str(out/'consumer/Cargo.toml'),'--release','--offline','--locked','--config',str(REPO/'aros-env.toml'),*options]
            for name,directory in [('libc','libc-source'),('polling','polling'),('async-io','async-io'),('async-net','async-net')]: command+=['--config','patch.crates-io.'+name+'.path='+json.dumps(str(out/directory))]
            for mode in ['normal','entropy-fail']:
                env=dict(os.environ,CARGO_HOME=str(args.cargo_home.resolve()),CARGO_TARGET_DIR=str(out/'target'),AROS_TLS_LIB_DIR=str(lib),AROS_TLS_RUN_ID=metadata['run_id'])
                env.pop('AROS_TLS_FORCE_FAIL',None)
                if mode=='entropy-fail': env['AROS_TLS_FORCE_FAIL']='1'
                run(command,env=env,cwd=out)
                target=out/('aros-https-'+mode); shutil.copy2(out/'target/x86_64-aros/release/aros-async-https',target)
                metadata['artifacts'][target.name]=sha(target)
            metadata['status']='built-not-run'; metadata['lock_sha256']=sha(out/'consumer/Cargo.lock')
        except BaseException:
            metadata['status']='failed'; save(); raise
    metadata['completed']=datetime.datetime.now(datetime.timezone.utc).isoformat(); save(); return metadata

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archives',type=Path); parser.add_argument('apkg',type=Path); parser.add_argument('output',type=Path)
    parser.add_argument('--toolchain',required=True); parser.add_argument('--cargo-home',type=Path,required=True); parser.add_argument('--profile',choices=['abort','unwind'],default='abort')
    args=parser.parse_args()
    try: print(json.dumps(build(args),indent=2))
    except (ValueError,OSError,subprocess.SubprocessError,tarfile.TarError) as error: parser.exit(1,str(error)+'\n')
if __name__=='__main__': main()
