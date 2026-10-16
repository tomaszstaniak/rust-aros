import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('baseline', ROOT/'ports/wat-server/baseline.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class BaselineReports(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.results=Path(self.tmp.name)
        source=ROOT/'docs/evidence/2026-10-15-wat-teardown/native-10'
        for path in source.glob('wl10-*'):shutil.copyfile(path,self.results/path.name)

    def test_real_completed_lifecycle_reports_are_accepted(self):
        self.assertEqual(module.check_results(self.results,'wl10','unwind',probes=False)['status'],'passed')

    def test_zero_rc_without_completed_trace_is_rejected(self):
        p=self.results/'wl10-active-exit.txt'
        p.write_text(p.read_text().replace('workers_joined','unobserved'))
        with self.assertRaisesRegex(ValueError,'workers_joined'):module.check_results(self.results,'wl10','unwind',probes=False)

    def test_missing_or_failed_case_never_becomes_success(self):
        p=self.results/'wl10-shutdown.rc';p.unlink()
        with self.assertRaises(ValueError):module.check_results(self.results,'wl10','unwind',probes=False)
        p.write_text('1\n')
        with self.assertRaises(ValueError):module.check_results(self.results,'wl10','unwind',probes=False)

    def test_absent_probe_reports_fail_full_unwind_suite(self):
        with self.assertRaises(ValueError):module.check_results(self.results,'wl10','unwind')

    def test_abort_profile_does_not_claim_cancellation_acceptance(self):
        result=module.check_results(self.results,'wl10','abort')
        self.assertEqual(result['cases'],['baseline'])

    def test_guest_payload_receipt_must_match_build(self):
        expected={'wat-server':'abc'}
        with self.assertRaises(ValueError):module.check_results(self.results,'wl10','abort',payload=expected)
        (self.results/'wl10-payload.json').write_text(json.dumps({'wat-server':'wrong'}))
        with self.assertRaises(ValueError):module.check_results(self.results,'wl10','abort',payload=expected)
        (self.results/'wl10-payload.json').write_text(json.dumps(expected))
        with self.assertRaises(ValueError):module.check_results(self.results,'wl10','abort',payload=expected)
        (self.results/'wl10-complete.json').write_text(json.dumps(expected))
        self.assertEqual(module.check_results(self.results,'wl10','abort',payload=expected)['status'],'passed')

    def test_native_profiles_reject_ambient_overrides_before_build(self):
        for profile in ('abort','unwind'):
            for variable in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_BUILD_RUSTFLAGS','CARGO_TARGET_X86_64_AROS_RUSTFLAGS','CARGO_PROFILE_RELEASE_PANIC'):
                env=dict(os.environ);env[variable]='-Cpanic=abort'
                result=subprocess.run([sys.executable,str(ROOT/'ports/wat-server/build.py'),str(self.results),
                    '--toolchain','unused','--aros','--panic-profile',profile],env=env,text=True,capture_output=True)
                self.assertNotEqual(result.returncode,0)
                self.assertIn('clear ambient',result.stderr)

    def test_build_cannot_write_inside_upstream(self):
        output=self.results/'generated'
        result=subprocess.run([sys.executable,str(ROOT/'ports/wat-server/baseline.py'),'build',
            str(self.results),str(output),'--toolchain','unused','--cargo-home',str(self.results),
            '--prefix','test'],text=True,capture_output=True)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('outside the pinned upstream',result.stderr)
        self.assertFalse(output.exists())

    def test_failed_recheck_replaces_previous_success(self):
        run=self.results/'run';run.mkdir()
        payload={'run-id':'unique-test-build'}
        (run/'run.json').write_text(json.dumps({'status':'built-not-run','prefix':'wl10','profile':'abort','payload_sha256':payload}))
        for suffix in ('payload','complete'):
            (self.results/('wl10-'+suffix+'.json')).write_text(json.dumps(payload))
        command=[sys.executable,str(ROOT/'ports/wat-server/baseline.py'),'check',str(run),str(self.results),
            '--machine','fixture','--runtime-profile','fixture']
        first=subprocess.run(command,text=True,capture_output=True)
        self.assertEqual(first.returncode,0,first.stderr)
        self.assertEqual(json.loads((run/'validation.json').read_text())['status'],'passed')
        (self.results/'wl10-rc.txt').write_text('20')
        second=subprocess.run(command,text=True,capture_output=True)
        self.assertNotEqual(second.returncode,0)
        self.assertEqual(json.loads((run/'validation.json').read_text())['status'],'failed')

    def test_dropped_trace_or_panic_is_rejected(self):
        p=self.results/'wl10-active-eof.txt'
        p.write_text(p.read_text()+'REQUEST_TRACE_DROPPED 1\n')
        with self.assertRaises(ValueError):module.check_results(self.results,'wl10','unwind',probes=False)

if __name__=='__main__':unittest.main()
