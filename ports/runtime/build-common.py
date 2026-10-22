#!/usr/bin/env python3
"""Run existing parking/Rayon probes with the guarded shared AROS compiler."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import sysroot_contract
spec = importlib.util.spec_from_file_location('cargo_main', ROOT / 'tools/build-cargo-main.py')
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('profile_build', type=Path)
    p.add_argument('sysroot', type=Path)
    p.add_argument('new_out', type=Path)
    p.add_argument('--case', choices=('parking', 'rayon'), required=True)
    p.add_argument('--pthread-objects', type=Path, help='already verified private detach correction')
    p.add_argument('--shutdown', action='store_true', help='opt-in Rayon patch only after reproducing exit failure')
    p.add_argument('--progress-prefix', help='private guest pathname for closed Rayon checkpoints')
    a = p.parse_args()
    if a.shutdown and a.case != 'rayon':
        p.error('shutdown is a Rayon-only variant')
    if any(os.environ.get(k) for k in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER')):
        p.error('clear ambient compiler flags and wrappers')
    base, sysroot, out = a.profile_build.absolute(), a.sysroot.absolute(), a.new_out.absolute()
    manifest = json.loads((base / 'build.json').read_text())
    profile = manifest['profile']
    contract = sysroot_contract.validate(sysroot, profile)
    if common.builder.tree_digest(Path(manifest['sdk'])) != manifest['sdk_sha256']:
        p.error('SDK changed')
    if subprocess.check_output([sysroot / 'bin/rustc', '-V'], text=True).strip() != manifest['compiler']:
        p.error('compiler changed')
    target = dict(manifest['target'])
    arm = profile == 'aarch64-v1-raspi'
    objects = []
    for name, expected in manifest['bridge_objects'].items():
        path = Path(name)
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            p.error('bridge object changed: ' + name)
        objects.append(str(path))
    if a.pthread_objects:
        objects += [str((a.pthread_objects / name).absolute()) for name in ('pthread.o','pthread_create.o')]
    out.mkdir(parents=True, exist_ok=False)
    source = out / 'consumer'
    shutil.copytree(ROOT / 'std-groundwork/runtime-regressions', source,
                    ignore=shutil.ignore_patterns('target','.cargo','.DS_Store'))
    features = []
    patch_options = ['--config', 'patch.crates-io.libc.path=' + json.dumps(str(sysroot / sysroot_contract.SOURCE / 'library/libc-aros'))]
    if a.shutdown:
        checksum='22e18b0f0062d30d4230b2e85ff77fdfe4326feb054b9783a3460d8435c8ab91'
        archives=list((Path(os.environ.get('CARGO_HOME',Path.home()/'.cargo'))/'registry/cache').glob('*/rayon-core-1.13.0.crate'))
        if len(archives)!=1 or hashlib.sha256(archives[0].read_bytes()).hexdigest()!=checksum:
            p.error('missing or changed pinned rayon-core archive')
        with tarfile.open(archives[0]) as tar:tar.extractall(out,filter='data')
        patched=out/'rayon-core-1.13.0'
        subprocess.run(['patch','--batch','--fuzz=0','-p1','-i',ROOT/'ports/runtime/patches/rayon.patch'],cwd=patched,check=True)
        patch_options += ['--config','patch.crates-io.rayon-core.path='+json.dumps(str(patched))]
        features=['--features','shutdown']
    if a.case=='parking':
        deadline=out/'deadline.o'
        subprocess.run([manifest['cc'],'--sysroot='+manifest['sdk'],'-O2','-c',ROOT/'ports/runtime/deadline.c','-o',deadline],check=True)
        objects.append(str(deadline))
    target['linker']=str(out/'linker')
    target['no-default-libraries']=False
    target['pre-link-args']={'gnu-cc':['--sysroot='+manifest['sdk']]+objects}
    target['late-link-args']={'gnu-cc':(['-lpthread'] if arm else ['-lcrt','-lnet','-lpthread'])+
                              (['-Wl,--wrap=pthread_cond_timedwait'] if a.case=='parking' else [])}
    common.write_linker(out/'linker',manifest['cc'])
    target_file=out/(profile+'.json');target_file.write_text(json.dumps(target,indent=2)+'\n')
    env=dict(os.environ,RUSTC=str(sysroot/'bin/rustc'),AROS_GCC=manifest['cc'],AROS_AR=manifest['ar'],
             AROS_SDK=manifest['sdk'],RPI_SDK=manifest['sdk'],RUNTIME_RUN_ID=out.name,
             RUNTIME_PARKER='generic' if arm else 'unix')
    if a.progress_prefix:
        env['RUNTIME_PROGRESS_PREFIX']=a.progress_prefix
    env['PATH']=str(sysroot/'bin')+os.pathsep+env.get('PATH','')
    command=[sysroot/'bin/cargo','build','--offline','--release','-Zjson-target-spec','-Zbuild-std=std,panic_abort',
             '--manifest-path',source/'Cargo.toml','--target',target_file,'--target-dir',out/'cargo',
             '--bin',a.case,*patch_options,*features]
    subprocess.run(command,env=env,check=True,timeout=600)
    binary=out/'cargo'/profile/'release'/a.case
    data=binary.read_bytes();common.builder.validate_elf(data,profile)
    if subprocess.check_output([manifest['nm'],'-u',binary],text=True).strip():
        p.error('undefined symbols')
    (out/a.case).write_bytes(data)
    metadata=dict(case=a.case,shutdown=a.shutdown,profile=profile,compiler=manifest['compiler'],
                  consumer_sha256={str(f.relative_to(source)):hashlib.sha256(f.read_bytes()).hexdigest() for f in source.rglob('*') if f.is_file()},
                  patch_sha256=hashlib.sha256((ROOT/'ports/runtime/patches/rayon.patch').read_bytes()).hexdigest() if a.shutdown else None,
                  sdk=manifest['sdk'],sdk_sha256=manifest['sdk_sha256'],sysroot_contract=contract,
                  target=target,command=list(map(str,command)),environment={k:env[k] for k in ('RUNTIME_RUN_ID','RUNTIME_PARKER','RUNTIME_PROGRESS_PREFIX') if k in env},
                  objects={name:hashlib.sha256(Path(name).read_bytes()).hexdigest() for name in objects},
                  artifact_sha256=hashlib.sha256(data).hexdigest(),runtime_acceptance=False)
    (out/'build.json').write_text(json.dumps(metadata,indent=2)+'\n')


if __name__=='__main__':main()
