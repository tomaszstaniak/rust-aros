"""Unsafe configurations must be rejected before creating a build directory."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'ports/runtime/build-common.py'


class RuntimeCommonPreflightTests(unittest.TestCase):
    def invoke(self, *options, flags=None):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            env = dict(os.environ)
            for key in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER'):
                env.pop(key, None)
            if flags:
                env.update(flags)
            result = subprocess.run([sys.executable, SCRIPT, root/'missing-profile',
                                     root/'missing-sysroot', root/'output', *options],
                                    env=env, capture_output=True, text=True, timeout=10)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root/'output').exists())
            self.assertNotIn('Traceback', result.stderr)
            return result.stderr

    def test_shutdown_cannot_change_parking_baseline(self):
        self.assertIn('Rayon-only', self.invoke('--case','parking','--shutdown'))

    def test_ambient_target_override_is_rejected(self):
        self.assertIn('ambient', self.invoke('--case','rayon',flags={'RUSTFLAGS':'--cfg unix'}))

    def test_ambient_wrapper_is_rejected(self):
        self.assertIn('ambient', self.invoke('--case','rayon',flags={'RUSTC_WRAPPER':'unknown'}))
