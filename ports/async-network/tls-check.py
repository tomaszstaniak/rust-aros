#!/usr/bin/env python3
"""Validate complete TLS evidence against a fresh build UUID and collected hashes."""
import argparse, collections, hashlib, json
from pathlib import Path

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def require(condition,message):
    if not condition: raise ValueError(message)
def validate_reports(run,normal,entropy,shell,records):
    start='START TLS Mbed3.6.7 jitterentropy3.7 x86_64-v11 run='+run
    finish='PASS HTTPS acceptance run='+run
    for name,text in [('normal',normal),('entropy',entropy)]:
        lines=text.splitlines()
        require(bool(lines) and lines[0]==start and lines[-1]==finish,name+' run identity/completion mismatch')
        require(lines.count(start)==1 and lines.count(finish)==1,name+' duplicate report')
    required=['PASS foreign TLS construction rejected; owner context preserved','PASS verified HTTPS with authenticated EOF','PASS pending handshake cancellation, live drop, and subsequent connection','PASS cleanup after rejection and concurrent TLS context lifetime','PASS raw transport EOF rejected']
    for line in required: require(normal.splitlines().count(line)==1,'missing or duplicated '+line)
    for port,flag in [(18771,4),(18772,1),(18771,8)]:
        prefix=f'PASS certificate rejection port={port} flag={flag} '
        require(sum(line.startswith(prefix) for line in normal.splitlines())==1,'missing certificate gate '+prefix)
    require(sum(line.startswith('PASS forced entropy failure before connect ') for line in entropy.splitlines())==1,'missing entropy failure gate')
    require(shell.splitlines()==[f'START {run}','0','0',f'DONE {run}'],'guest shell failure or stale shell report')
    require(records and all(r.get('run_id')==run for r in records),'fixture run identity mismatch')
    kinds=collections.Counter(r.get('kind') for r in records)
    require(kinds==dict(ready=1,completed=1,**{'verified-http':4,'certificate-alert':3,'truncated':1,'cancelled-handshake':1,'live-drop':1}),'fixture event counts mismatch')
    require(records[0]['kind']=='ready' and records[-1]['kind']=='completed','fixture lifecycle incomplete')
    end=records[-1]
    require(end.get('accepted')==10 and end.get('entropy_connections')==0 and end.get('errors')==[],'fixture failure or entropy connection')
    alerts=collections.Counter(r.get('port') for r in records if r['kind']=='certificate-alert')
    require(alerts=={18771:2,18772:1},'certificate alert ports mismatch')

def check(build,results,prefix,fixture):
    metadata=json.loads((build/'tls-build.json').read_text()); require(metadata['status']=='built-not-run','build did not finish')
    for name,expected in metadata['artifacts'].items(): require(sha(build/name)==expected,'binary changed: '+name)
    for name,expected in {**metadata['sources'],**metadata['adapter']}.items(): require(sha(build/name)==expected,'build source changed: '+name)
    collection={}
    for line in (results/'COLLECT-MANIFEST.sha256').read_text().splitlines():
        digest,name=line.split(None,1); collection[name.lstrip('*').removeprefix('./')]=digest
    reports=[]
    for suffix in ['.log','-entropy.log','-shell.log']:
        name=prefix+suffix; path=results/name
        require(collection.get(name)==sha(path),'collected report checksum mismatch: '+name)
        reports.append(path.read_text())
    records=[json.loads(line) for line in fixture.read_text().splitlines()]
    validate_reports(metadata['run_id'],*reports,records)
    return {'status':'passed','run_id':metadata['run_id'],'profile':metadata['profile'],'build_manifest_sha256':sha(build/'tls-build.json'),'binaries':metadata['artifacts'],'reports':{prefix+s:sha(results/(prefix+s)) for s in ['.log','-entropy.log','-shell.log']},'fixture_sha256':sha(fixture)}
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('build',type=Path);parser.add_argument('results',type=Path);parser.add_argument('prefix');parser.add_argument('fixture',type=Path);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    try:
        if args.output.exists(): raise ValueError('refusing existing validation output')
        value=check(args.build,args.results,args.prefix,args.fixture)
        with args.output.open('x') as f: f.write(json.dumps(value,indent=2)+'\n')
        print(json.dumps(value,indent=2))
    except (ValueError,OSError,KeyError) as error: parser.exit(1,str(error)+'\n')
if __name__=='__main__': main()
