#!/usr/bin/env python3
"""Check retained before/after runtime evidence, not build success alone."""
import argparse
import hashlib
import json
import re
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def after_receipts(observation, identity):
    require(observation['run_id'] == identity, 'stale observation')
    require(observation['status'] == 'passed' and not observation['errors'], 'observation failed')
    require(set(observation['cases']) == {'detach', 'parking', 'rayon'}, 'missing case')
    for name, case in observation['cases'].items():
        require(case['status'] == 'passed' and case.get('returncode') == 0 and
                0 <= case.get('elapsed', -1) <= 60 and case.get('started', -1) >= 0,
                'missing successful bounded Shell exit: ' + name)


def baseline_case(case, receipt, log):
    require(receipt['status'] == 'timeout' and receipt.get('started', -1) >= 0, 'baseline did not reproduce: ' + case)
    if case == 'detach':
        require('PASS detach-before cycles=64' in log and 'FAIL leaked-slot phase=1 cycle=0' in log, 'wrong detach baseline failure')
    if case == 'parking':
        deltas = [int(n) for n in re.findall(r'TIMEDWAIT realtime_delta_seconds=(-?\d+)', log)]
        require(deltas and any(abs(n) > 86400 for n in deltas), 'no reproduced clock-domain mismatch')
    if case == 'rayon':
        require('PASS private-pool worker-exits=2' in log and 'rayon before-return' in log, 'wrong Rayon baseline failure')


def collected_files(root):
    manifest = {}
    for line in (root / 'COLLECT-MANIFEST.sha256').read_text().splitlines():
        digest, path = line.split(None, 1)
        manifest[path.lstrip('*').removeprefix('./')] = digest
    return manifest


def collection(root, name):
    manifest = collected_files(root)
    file = root / name
    require(file.is_file() and manifest.get(name) == sha(file), 'missing or changed collection: ' + name)
    return file


def check(run, before, after, observations):
    m = json.loads((run/'build.json').read_text())
    require(m['status'] == 'built-not-run' and m['profile'] == 'unwind' and m['sdk_object_matches'] == 33, 'incomplete build')
    for name, digest in m['input_sha256'].items():
        require(sha(run/name) == digest, 'retained source/object changed: ' + name)
    for name, digest in m['payload_sha256'].items():
        require(sha(run/'payload'/name) == digest, 'payload changed: ' + name)
    old = json.loads((observations/'before.json').read_text())
    new = json.loads((observations/'after.json').read_text())
    require(old['run_id'] == m['variants']['before']['identity'] and not old['errors'], 'invalid baseline observation')
    require(set(old['cases']) == {'detach','parking','rayon'}, 'missing baseline case')
    after_receipts(new, m['variants']['after']['identity'])
    reports = {}
    for variant, root in [('before', before), ('after', after)]:
        identity = m['variants'][variant]['identity']
        names = m['variants'][variant]['reports']
        for case in ('detach','parking','rayon'):
            logfile = next(n for n in names if n.endswith('-' + case + '.log'))
            log = collection(root, logfile).read_text(errors='replace')
            require('RUN ' + identity + ' ' + case + ' begin' in log, 'missing native identity: ' + logfile)
            reports[variant + '/' + logfile] = sha(root/logfile)
            if variant == 'after':
                rcfile = logfile[:-4] + '.rc'
                require(collection(root, rcfile).read_text().strip() == '0', 'guest exit failed')
                reports[variant + '/' + rcfile] = sha(root/rcfile)
                required = {'detach': ['PASS detach-before cycles=64','PASS detach-after cycles=64','detach pass'],
                            'parking': ['PASS timeout','parking pass wakeups=8'],
                            'rayon': ['PASS private-pool worker-exits=2','PASS global-pool worker-exits=2','rayon before-return']}[case]
                require(all(marker in log for marker in required) and 'FAIL' not in log and 'panicked' not in log, 'native case failed: ' + case)
            else:
                rcfile = logfile[:-4] + '.rc'
                require(not (root/rcfile).exists() and not (root/rcfile).is_symlink() and
                        rcfile not in collected_files(root), 'baseline has contradictory Shell exit evidence: ' + rcfile)
                baseline_case(case, old['cases'][case], log)
    return {'status':'passed','run_id':m['run_id'],'profile':m['profile'],
            'build_sha256':sha(run/'build.json'),'reports':reports,
            'observations':{v:sha(observations/(v+'.json')) for v in ('before','after')}}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('run','before_results','after_results','observations','output'):
        p.add_argument(name,type=Path)
    a=p.parse_args()
    require(not a.output.exists(), 'validation output must be new')
    result=check(a.run,a.before_results,a.after_results,a.observations)
    with a.output.open('x') as f:
        f.write(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))
if __name__=='__main__':main()
