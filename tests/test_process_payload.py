from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / 'std-groundwork/process-stdio/tools/stage-payload.py'
class ProcessPayload(unittest.TestCase):
    def test_payload_has_execute_permissions_and_guest_protection_step(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); source = root / 'source'; source.write_bytes(b'fixture'); source.chmod(0o644)
            dest = root / 'payload'
            result = subprocess.run([sys.executable, str(TOOL), str(source), str(dest), '--prefix', 'probe'], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual((dest / 'process-stdio').read_bytes(), b'fixture')
            self.assertEqual((dest / 'process-stdio').stat().st_mode & 0o111, 0o111)
            script = (dest / 'run-tests').read_text()
            self.assertLess(script.index('Protect RAM:pr/process-stdio +e'), script.index('--case'))
            # Re-running must not replace an existing payload.
            result = subprocess.run([sys.executable, str(TOOL), str(source), str(dest), '--prefix', 'probe'], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual((dest / 'process-stdio').read_bytes(), b'fixture')
    def test_prefix_cannot_inject_shell_commands(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); source = root / 'source'; source.write_bytes(b'fixture')
            result = subprocess.run([sys.executable, str(TOOL), str(source), str(root / 'out'), '--prefix', 'x\nDelete SYS: ALL'], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root / 'out').exists())
