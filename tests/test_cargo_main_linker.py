import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('cargo_main', ROOT / 'tools/build-cargo-main.py')
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class CargoMainLinkerTests(unittest.TestCase):
    def test_crt_sections_are_retained_and_compiler_status_propagates(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "compiler with ' quote"
            compiler.write_text('#!/bin/sh\nprintf "%s\\n" "$@"\nexit 7\n')
            compiler.chmod(0o755)
            linker = root / 'linker'
            builder.write_linker(linker, compiler)
            result = subprocess.run([linker, 'a b.o', '-Wl,--gc-sections'],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 7)
            self.assertEqual(result.stdout.splitlines(),
                             ['a b.o', '-Wl,--gc-sections', '-Wl,--no-gc-sections'])
