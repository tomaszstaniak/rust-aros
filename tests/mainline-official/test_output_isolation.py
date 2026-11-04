from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ImportOutputIsolationTests(unittest.TestCase):
    def test_import_generation_cannot_write_inside_sdk_or_source(self):
        for inside in ('sdk', 'source'):
            with self.subTest(inside=inside), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); source = root/'source'; sdk = root/'sdk'; sdk.mkdir()
                config = source/'compiler/crt/posixc/posixc.conf'; config.parent.mkdir(parents=True); config.write_text('config')
                tool = root/'tool'; tool.write_text('#!/bin/sh\nexit 0\n'); tool.chmod(0o755)
                output = root/inside/'new-output'
                command = ['python3', str(ROOT/'tools/official-posix-imports.py'), '--source', str(source),
                           '--sdk', str(sdk), '--cc', str(tool), '--ar', str(tool), '--genmodule', str(tool), '--output', str(output)]
                result = subprocess.run(command, capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(output.exists(), result.stderr.decode())
