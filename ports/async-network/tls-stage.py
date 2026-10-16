#!/usr/bin/env python3
"""Create a new TLS payload with public test CAs, exact build identity and finite guest gates."""
import argparse, hashlib, json, re, shutil
from pathlib import Path

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def stage(build,certificates,output,guest,prefix):
    if output.exists() or output.is_symlink(): raise ValueError('refusing existing payload output')
    if not re.fullmatch(r'RAM:[A-Za-z0-9_/-]+',guest) or '..' in guest.split('/'): raise ValueError('use a private RAM: guest directory')
    if not re.fullmatch(r'[a-z0-9-]+',prefix): raise ValueError('use a unique lowercase result prefix')
    metadata=json.loads((build/'tls-build.json').read_text())
    if metadata['status']!='built-not-run': raise ValueError('build must have completed')
    for name,digest in metadata['artifacts'].items():
        if sha(build/name)!=digest: raise ValueError('binary checksum mismatch '+name)
    certificates_manifest=json.loads((certificates/'fixture.json').read_text())
    for name in ['root.pem','other.pem']:
        if sha(certificates/name)!=certificates_manifest['cert_sha256'][name]: raise ValueError('certificate checksum mismatch '+name)
    output.mkdir(parents=True)
    for name in ['aros-https-normal','aros-https-entropy-fail']: shutil.copy2(build/name,output/name)
    for name in ['root.pem','other.pem']: shutil.copy2(certificates/name,output/name)
    run=metadata['run_id']
    lines=['FailAt 21','Stack 1048576',f'Echo "START {run}" >RESULTS:{prefix}-shell.log']
    for mode,suffix,extra in [('normal','',''),('entropy-fail','-entropy',' entropy-fail')]:
        lines += [f'{guest}/aros-https-{mode} 10.0.2.2 {guest}/root.pem {guest}/other.pem{extra} >RESULTS:{prefix}{suffix}.log',f'Echo $RC >>RESULTS:{prefix}-shell.log']
    lines += [f'Echo "DONE {run}" >>RESULTS:{prefix}-shell.log']
    (output/'run-https').write_text('\n'.join(lines)+'\n')
    payload={'run_id':run,'profile':metadata['profile'],'build_manifest_sha256':sha(build/'tls-build.json'),'guest_directory':guest,'result_prefix':prefix,'files':{p.name:sha(p) for p in sorted(output.iterdir())}}
    (output/'payload.json').write_text(json.dumps(payload,indent=2)+'\n')
    return payload

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['build','certificates','output']: parser.add_argument(name,type=Path)
    parser.add_argument('--guest-dir',required=True);parser.add_argument('--prefix',required=True);args=parser.parse_args()
    try: print(json.dumps(stage(args.build,args.certificates,args.output,args.guest_dir,args.prefix),indent=2))
    except (ValueError,OSError,KeyError) as error: parser.exit(1,str(error)+'\n')
if __name__=='__main__':main()
