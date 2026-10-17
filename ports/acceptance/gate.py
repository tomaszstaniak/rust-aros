#!/usr/bin/env python3
"""Fresh private v11 socket/async/TLS/entropy/process/LSP/mmap acceptance bundle."""
import argparse,hashlib,importlib.util,json,os,re,shutil,signal,subprocess,sys,time,uuid
from pathlib import Path
REPO=Path(__file__).resolve().parents[2]
PY=sys.executable

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def require(ok,message):
    if not ok:raise ValueError(message)
def module(name,path):
    spec=importlib.util.spec_from_file_location(name,path);mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod);return mod
def receipts(results,prefix,expected):
    for name in ['start','done']:
        try: value=json.loads((results/(prefix+'-'+name+'.json')).read_text())
        except (OSError,ValueError) as error:raise ValueError('missing/invalid '+name+' receipt') from error
        require(value==expected,'stale '+name+' receipt')
def run(command,log,env=None):
    with log.open('w') as stream:subprocess.run(list(map(str,command)),env=env,stdout=stream,stderr=subprocess.STDOUT,check=True)
def probe_options(profile,package):
    sys.path.insert(0,str(REPO/'ports/wat-server'))
    from native_profile import native_options
    options=native_options(REPO,profile,os.environ)
    if package=='entropy-std':
        # Preserve this example's supported custom backend when bypassing its .cargo config.
        flags=json.loads(options[-1].split('=',1)[1])+['--cfg','getrandom_backend="custom"']
        options[-1]='build.rustflags='+json.dumps(flags)
    return options

def build(args):
    config=json.loads(args.config.read_text());out=args.output.resolve()
    require(not out.exists() and not out.is_relative_to(REPO),'select a new private output')
    out.mkdir(parents=True);(out/'logs').mkdir();runid=str(uuid.uuid4());prefix='rg'+runid.replace('-','')[:12]
    profile=config.get('profile','unwind');tc=config['toolchain'];cargo=config['cargo_home']
    common=['--toolchain',tc,'--cargo-home',cargo,'--profile',profile]
    metadata={'status':'building','run_id':runid,'prefix':prefix,'profile':profile,'configuration':config}
    def save():(out/'suite.json').write_text(json.dumps(metadata,indent=2)+'\n')
    save()
    try:
        run([PY,REPO/'ports/async-network/tls-build.py',config['archives'],config['apkg'],out/'tls',*common],out/'logs/tls.log')
        run([PY,REPO/'ports/async-network/build.py',config['archives'],out/'async',*common],out/'logs/async.log')
        run([PY,REPO/'ports/wat-server/baseline.py','build',config['wat'],out/'lsp',*common,'--prefix',prefix+'l'],out/'logs/lsp.log')
        run([PY,REPO/'ports/mmap/build.py',out/'mmap','--font',config['font'],'--font-license',config['font_license'],'--toolchain',tc,'--cargo-home',config.get('mmap_cargo_home',cargo),'--profile',profile],out/'logs/mmap.log')
        run([PY,REPO/'ports/async-network/tls-fixture.py',out/'certificates','--prepare'],out/'logs/certificates.log')
        payload=out/'payload';shutil.copytree(out/'lsp/payload',payload)
        run([PY,REPO/'ports/async-network/tls-stage.py',out/'tls',out/'certificates',payload/'tls','--guest-dir','RAM:wl/tls','--prefix',prefix+'t'],out/'logs/tls-stage.log')
        shutil.copytree(out/'mmap/payload',payload/'mm')
        p=payload/'mm/run-tests';p.write_text(p.read_text().replace('RAM:mm/','RAM:wl/mm/'))
        shutil.copy2(out/'async/target/x86_64-aros/release/aros-async-consumer',payload/'async-check')
        root=out/'probes';(root/'std-groundwork').mkdir(parents=True)
        shutil.copytree(out/'tls/libc-source',root/'std-groundwork/libc-aros')
        shutil.copytree(REPO/'transport',root/'transport',ignore=shutil.ignore_patterns('target','.cargo'))
        sys.path.insert(0,str(REPO/'ports/wat-server'));from native_profile import native_options
        env=dict(os.environ,CARGO_HOME=cargo,CARGO_TARGET_DIR=str(out/'probe-target'))
        for package,binary,target in [('async-network','aros-async-network','socket-check'),('entropy-std','entropy-std','entropy-check'),('process-stdio','process-stdio','process-stdio')]:
            shutil.copytree(REPO/'std-groundwork'/package,root/'std-groundwork'/package,ignore=shutil.ignore_patterns('target','.cargo'))
            command=['cargo','+'+tc,'build','--release','--offline','--manifest-path',root/'std-groundwork'/package/'Cargo.toml','--config',REPO/'aros-env.toml',*probe_options(profile,package),'--config','patch.crates-io.libc.path='+json.dumps(str(root/'std-groundwork/libc-aros'))]
            run(command,out/'logs'/('probe-'+package+'.log'),env)
            shutil.copy2(out/'probe-target/x86_64-aros/release'/binary,payload/target)
        # All paths must compile the same compatibility boundary.
        boundary=module('baseline_source',REPO/'ports/wat-server/baseline.py')
        expected=boundary.source_hash(out/'tls/libc-source')
        for tree in [out/'async/libc-source',out/'lsp/libc-source',out/'mmap/libc-source',root/'std-groundwork/libc-aros']:
            require(boundary.source_hash(tree)==expected,'libc sources differ across components')
        metadata['libc_source_sha256']=expected
        lines=['FailAt 21','Stack 1048576']
        # Refuse all results before publishing a fresh start receipt.
        names={prefix+'-start.json',prefix+'-done.json'}
        for script in [payload/'run-tests',payload/'tls/run-https',payload/'mm/run-tests']:
            names.update(re.findall(r'RESULTS:([a-zA-Z0-9_.-]+)',script.read_text()))
        for kind in ['socket','async','entropy']:
            names.update([prefix+'-'+kind+'.log',prefix+'-'+kind+'.rc'])
        for case in ['status','output','args','spawn-error','lsp-bytewise']:
            names.update([prefix+'-process-'+case+'.txt',prefix+'-process-'+case+'.rc'])
        for name in sorted(names):lines += ['If EXISTS RESULTS:'+name,'Echo "Refusing existing combined evidence"','Quit 20','EndIf']
        lines+=['Copy RAM:wl/gate-identity.json RESULTS:'+prefix+'-start.json']
        for kind,cmd in [('socket','RAM:wl/socket-check 10.0.2.2:18763'),('async','RAM:wl/async-check 10.0.2.2:18761 10.0.2.2:18762'),('entropy','RAM:wl/entropy-check')]:
            lines += [cmd+' >RESULTS:'+prefix+'-'+kind+'.log','Echo $RC >RESULTS:'+prefix+'-'+kind+'.rc']
        for case in ['status','output','args','spawn-error','lsp-bytewise']:
            lines += ['RAM:wl/process-stdio --case '+case+' --report RESULTS:'+prefix+'-process-'+case+'.txt','Echo $RC >RESULTS:'+prefix+'-process-'+case+'.rc']
        lines += ['Execute RAM:wl/mm/run-tests','Execute RAM:wl/tls/run-https','Execute RAM:wl/run-tests','Copy RAM:wl/gate-identity.json RESULTS:'+prefix+'-done.json']
        (payload/'run-all').write_text('\n'.join(lines)+'\n')
        identity={'run_id':runid,'files':{p.relative_to(payload).as_posix():sha(p) for p in sorted(payload.rglob('*')) if p.is_file()}}
        (payload/'gate-identity.json').write_text(json.dumps(identity,indent=2)+'\n')
        metadata.update(status='built-not-run',identity=identity,reports=sorted(names),input_sha256={p.relative_to(out).as_posix():sha(p) for tree in ['tls','async','lsp','mmap','probes'] for p in sorted((out/tree).rglob('*')) if p.is_file() and not any(part in ('target','build','probe-target','logs') for part in p.relative_to(out/tree).parts)})
        save();return metadata
    except BaseException:
        metadata['status']='failed';save();raise

def fixtures(args):
    out=args.run.resolve();meta=json.loads((out/'suite.json').read_text());directory=out/'fixtures';directory.mkdir()
    require(meta['status']=='built-not-run','build incomplete')
    tls=json.loads((out/'tls/tls-build.json').read_text());children={};files={};stop=False
    def finish(*_):nonlocal stop;stop=True
    signal.signal(signal.SIGINT,finish);signal.signal(signal.SIGTERM,finish)
    commands={'socket':[PY,REPO/'ports/async-network/fixture.py','--port','18763','--refused-port','18764','--connections','83','--empty-connections','2','--deadline','300'],'async':[PY,REPO/'ports/async-network/fixture.py','--deadline','300'],'tls':[PY,REPO/'ports/async-network/tls-fixture.py',out/'certificates','--run-id',tls['run_id'],'--deadline','300']}
    try:
        for name,cmd in commands.items():
            files[name]=(directory/(name+'.log')).open('w');children[name]=subprocess.Popen(list(map(str,cmd)),stdout=files[name],stderr=subprocess.STDOUT)
        print('Fixtures started; inspect ready records before guest Execute. SIGINT only after complete guest run.',flush=True)
        deadline=time.monotonic()+300
        while not stop and time.monotonic()<deadline:
            if any(p.poll() not in (None,0) for p in children.values()):break
            time.sleep(.1)
        if children['tls'].poll() is None:children['tls'].send_signal(signal.SIGINT)
        codes={}
        for name,child in children.items():
            try:codes[name]=child.wait(timeout=6)
            except subprocess.TimeoutExpired:child.terminate();codes[name]=child.wait(timeout=3)
        result={'status':'completed' if stop and all(c==0 for c in codes.values()) else 'failed','run_id':meta['run_id'],'returncodes':codes}
        (directory/'status.json').write_text(json.dumps(result,indent=2)+'\n');require(result['status']=='completed','fixtures failed; inspect logs')
    finally:
        for child in children.values():
            if child.poll() is None:child.terminate();child.wait(timeout=3)
        for stream in files.values():stream.close()

def check(args):
    out=args.run.resolve();results=args.results.resolve();m=json.loads((out/'suite.json').read_text());require(m['status']=='built-not-run','build incomplete')
    require(not (out/'validation.json').exists(),'refusing existing validation')
    for name,digest in m['input_sha256'].items():require(sha(out/name)==digest,'retained build input changed: '+name)
    for name,digest in m['identity']['files'].items():require(sha(out/'payload'/name)==digest,'payload changed: '+name)
    collected={}
    for line in (results/'COLLECT-MANIFEST.sha256').read_text().splitlines():
        digest,name=line.split(None,1);collected[name.lstrip('*').removeprefix('./')]=digest
    for name in m['reports']:require(collected.get(name)==sha(results/name),'collection mismatch: '+name)
    receipts(results,m['prefix'],m['identity'])
    prefix=m['prefix'];fixtures=json.loads((out/'fixtures/status.json').read_text());require(fixtures=={'status':'completed','run_id':m['run_id'],'returncodes':{'socket':0,'async':0,'tls':0}},'fixtures incomplete or stale')
    def report(kind,marker):
        require((results/(prefix+'-'+kind+'.rc')).read_text().strip()=='0',kind+' failed')
        require(marker in (results/(prefix+'-'+kind+('.txt' if kind.startswith('process-') else '.log'))).read_text(),kind+' missing result')
    for name,marker in [('socket','PASS socket boundary acceptance'),('async','PASS real async consumer acceptance'),('entropy','STD_ENTROPY_OK')]:report(name,marker)
    for case in ['status','output','args','spawn-error','lsp-bytewise']:report('process-'+case,'CASE_DONE case='+case+' result=pass')
    for name,count in [('socket',83),('async',88)]:
        rows=[json.loads(line) for line in (out/'fixtures'/(name+'.log')).read_text().splitlines()];require(rows[-1]=={'completed':count,'errors':[]},name+' fixture incomplete')
    mm=json.loads((out/'mmap/build.json').read_text());p=mm['prefix'];require((results/(p+'.rc')).read_text().strip()=='0','mmap failed');require(('PASS mmap font acceptance run='+mm['run_id']) in (results/(p+'.log')).read_text(),'mmap stale/incomplete')
    base=module('gate_baseline',REPO/'ports/wat-server/baseline.py');lsp=json.loads((out/'lsp/run.json').read_text());lsp_result=base.check_results(results,lsp['prefix'],m['profile'],payload=lsp['payload_sha256'])
    tls=module('gate_tls',REPO/'ports/async-network/tls-check.py');tls_result=tls.check(out/'tls',results,prefix+'t',out/'fixtures/tls.log')
    value={'status':'passed','run_id':m['run_id'],'profile':m['profile'],'machine':args.machine,'lsp_cases':lsp_result['cases'],'tls':tls_result,'suite_sha256':sha(out/'suite.json'),'reports':{name:sha(results/name) for name in m['reports']},'fixtures':{p.name:sha(p) for p in (out/'fixtures').iterdir() if p.is_file()}}
    with (out/'validation.json').open('x') as stream:stream.write(json.dumps(value,indent=2)+'\n')
    return value

def main():
    p=argparse.ArgumentParser(description=__doc__);commands=p.add_subparsers(dest='command',required=True)
    b=commands.add_parser('build');b.add_argument('config',type=Path);b.add_argument('output',type=Path)
    f=commands.add_parser('fixtures');f.add_argument('run',type=Path)
    c=commands.add_parser('check');c.add_argument('run',type=Path);c.add_argument('results',type=Path);c.add_argument('--machine',required=True)
    args=p.parse_args()
    try:
        result={'build':build,'fixtures':fixtures,'check':check}[args.command](args)
        if result:print(json.dumps({'status':result['status'],'run_id':result['run_id']},indent=2))
    except (OSError,ValueError,KeyError,subprocess.SubprocessError) as error:p.exit(1,str(error)+'\n')
if __name__=='__main__':main()
