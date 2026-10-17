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
