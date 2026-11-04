import importlib.util
from pathlib import Path
import unittest

p=Path(__file__).resolve().parents[1]/'ports/runtime/check.py'
s=importlib.util.spec_from_file_location('runtime_check',p)
check=importlib.util.module_from_spec(s);s.loader.exec_module(check)

class RuntimeValidationTests(unittest.TestCase):
    def observation(self):
        return {'run_id':'run-after','status':'passed','errors':[],
                'cases':{n:{'status':'passed','returncode':0,'started':1,'elapsed':.1} for n in ('detach','parking','rayon')}}
    def test_actual_exits_required_for_all_cases(self):
        check.after_receipts(self.observation(),'run-after')
    def test_top_level_pass_cannot_hide_missing_exit(self):
        o=self.observation();o['cases']['rayon']={'status':'timeout'}
        with self.assertRaises(ValueError):check.after_receipts(o,'run-after')
    def test_stale_run_rejected(self):
        with self.assertRaises(ValueError):check.after_receipts(self.observation(),'other-after')
    def test_nonzero_exit_rejected(self):
        o=self.observation();o['cases']['detach']['returncode']=20
        with self.assertRaises(ValueError):check.after_receipts(o,'run-after')
    def test_errors_rejected(self):
        o=self.observation();o['errors']=['late receipt']
        with self.assertRaises(ValueError):check.after_receipts(o,'run-after')
    def test_rayon_baseline_nonzero_exit_does_not_prove_hang(self):
        with self.assertRaises(ValueError):
            check.baseline_case('rayon', {'status':'failed','started':1,'returncode':20},
                                'PASS private-pool worker-exits=2\nrayon before-return\n')
    def test_parking_baseline_normal_deadline_does_not_prove_clock_bug(self):
        with self.assertRaises(ValueError):
            check.baseline_case('parking', {'status':'timeout','started':1},
                                'TIMEDWAIT realtime_delta_seconds=1\n')
    def test_parking_baseline_clock_mismatch_and_hang(self):
        check.baseline_case('parking', {'status':'timeout','started':1},
                            'TIMEDWAIT realtime_delta_seconds=-1792235020\n')
    def test_rayon_baseline_hang_after_main(self):
        check.baseline_case('rayon', {'status':'timeout','started':1},
                            'PASS private-pool worker-exits=2\nrayon before-return\n')

class RuntimeBaselineExitFileTests(unittest.TestCase):
    def setUp(self):
        import tempfile
        import json
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.run, self.before, self.after, self.obs = [self.root/n for n in ('run','before','after','obs')]
        for p in (self.run,self.before,self.after,self.obs):p.mkdir()
        cases = ('detach','parking','rayon')
        diagnostics = {
            'detach':'PASS detach-before cycles=64\nFAIL leaked-slot phase=1 cycle=0\n',
            'parking':'TIMEDWAIT realtime_delta_seconds=-1792235020\n',
            'rayon':'PASS private-pool worker-exits=2\nrayon before-return\n'}
        green = {
            'detach':'PASS detach-before cycles=64\nPASS detach-after cycles=64\ndetach pass\n',
            'parking':'PASS timeout\nparking pass wakeups=8\n',
            'rayon':'PASS private-pool worker-exits=2\nPASS global-pool worker-exits=2\nrayon before-return\n'}
        manifest = {'status':'built-not-run','run_id':'run','profile':'unwind','sdk_object_matches':33,
                    'input_sha256':{},'payload_sha256':{},'variants':{}}
        for variant, directory, logs in [('before',self.before,diagnostics),('after',self.after,green)]:
            names=[]
            for case in cases:
                prefix=variant+'-'+case
                names.extend([prefix+'.log',prefix+'.rc'])
                (directory/(prefix+'.log')).write_text(f'RUN run-{variant} {case} begin\n'+logs[case])
                if variant=='after':(directory/(prefix+'.rc')).write_text('0\n')
            manifest['variants'][variant]={'identity':'run-'+variant,'reports':names}
            self.collection_manifest(directory)
        (self.run/'build.json').write_text(json.dumps(manifest))
        (self.obs/'before.json').write_text(json.dumps({'run_id':'run-before','errors':[],
            'cases':{case:{'status':'timeout','started':1} for case in cases}}))
        (self.obs/'after.json').write_text(json.dumps(RuntimeValidationTests().observation()))

    def collection_manifest(self, directory):
        import hashlib
        entries=[f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n'
                 for p in sorted(directory.iterdir()) if p.name!='COLLECT-MANIFEST.sha256']
        (directory/'COLLECT-MANIFEST.sha256').write_text(''.join(entries))

    def validate(self):return check.check(self.run,self.before,self.after,self.obs)

    def test_missing_baseline_exit_files_preserves_valid_run(self):
        self.assertEqual(self.validate()['status'],'passed')

    def test_baseline_exit_file_rejects_network_timeout_for_every_case(self):
        for case in ('detach','parking','rayon'):
            for content in ('0\n','20\n','invalid\n',''):
                with self.subTest(case=case,content=content):
                    rc=self.before/('before-'+case+'.rc');rc.write_text(content)
                    self.collection_manifest(self.before)
                    try:
                        with self.assertRaisesRegex(ValueError,'baseline.*exit'):
                            self.validate()
                    finally:rc.unlink();self.collection_manifest(self.before)

    def test_unmanifested_exit_file_also_rejects(self):
        (self.before/'before-rayon.rc').write_text('0\n')
        with self.assertRaisesRegex(ValueError,'baseline.*exit'):self.validate()

    def test_manifested_exit_cannot_be_hidden_by_missing_file(self):
        rc=self.before/'before-rayon.rc';rc.write_text('0\n')
        self.collection_manifest(self.before);rc.unlink()
        with self.assertRaisesRegex(ValueError,'baseline.*exit'):self.validate()
