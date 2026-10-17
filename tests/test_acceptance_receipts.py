import importlib.util,json,tempfile,unittest
from pathlib import Path

class ReceiptTests(unittest.TestCase):
    def module(self):
        path=Path(__file__).resolve().parents[1]/'ports/acceptance/gate.py'
        self.assertTrue(path.exists(),'combined gate is missing')
        spec=importlib.util.spec_from_file_location('acceptance_gate',path);mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod);return mod
    def test_rejects_stale_completion_even_with_fresh_start(self):
        gate=self.module()
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); expected={'run_id':'new','files':{'app':'abc'}}
            (root/'r-start.json').write_text(json.dumps(expected));(root/'r-done.json').write_text(json.dumps({'run_id':'old','files':{'app':'abc'}}))
            with self.assertRaises(ValueError):gate.receipts(root,'r',expected)
    def test_requires_both_matching_receipts(self):
        gate=self.module()
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);expected={'run_id':'unique','files':{'app':'abc'}}
            for name in ['start','done']:(root/f'r-{name}.json').write_text(json.dumps(expected))
            gate.receipts(root,'r',expected)
            (root/'r-done.json').unlink()
            with self.assertRaises(ValueError):gate.receipts(root,'r',expected)

    def test_entropy_build_keeps_explicit_backend_and_unwind(self):
        gate=self.module()
        command=gate.probe_options('unwind','entropy-std')
        flags=json.loads(command[-1].split('=',1)[1])
        self.assertIn('getrandom_backend="custom"',flags)
        self.assertIn('-Cpanic=unwind',flags)
        self.assertNotIn('getrandom_backend="custom"',json.loads(gate.probe_options('unwind','process-stdio')[-1].split('=',1)[1]))
