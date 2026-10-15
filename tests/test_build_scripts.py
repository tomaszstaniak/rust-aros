import json
import os
from pathlib import Path
import subprocess
import shutil
import tomllib
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class BuildScripts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.compiled = tempfile.TemporaryDirectory(prefix='aros-build-scripts-')
        cls.executables = {}
        for relative in ('aros', 'std-groundwork/libc-aros'):
            dest = Path(cls.compiled.name) / relative.replace('/', '-')
            subprocess.run(['rustc', '--edition=2021', str(ROOT / relative / 'build.rs'), '-o', str(dest)], check=True)
            cls.executables[relative] = dest

    @classmethod
    def tearDownClass(cls):
        cls.compiled.cleanup()

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='aros tools ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        # Replacing every 'gcc' in this path cannot locate the real archiver.
        tools = self.root / 'gcc-parent'
        tools.mkdir()
        self.compiler = tools / 'custom-cc'
        self.archiver = tools / 'custom-ar'
        for tool in (self.compiler, self.archiver):
            tool.write_text('#!/bin/sh\npython3 - "$0" "$@" <<\'END\'\nimport json, os, sys\nwith open(os.environ["CALL_LOG"], "a") as f: f.write(json.dumps(sys.argv[1:]) + "\\n")\nEND\n')
            tool.chmod(0o755)
        self.log = self.root / 'calls'
        self.env = {k: v for k, v in os.environ.items() if not k.startswith('AROS_')}
        self.env.update(AROS_GCC=str(self.compiler), AROS_AR=str(self.archiver),
                        AROS_SDK=str(self.root / 'sdk'), OUT_DIR=str(self.root),
                        TARGET='x86_64-aros', CALL_LOG=str(self.log))

    def run_script(self, relative):
        self.env['CARGO_MANIFEST_DIR'] = str(ROOT / relative)
        return subprocess.run([str(self.executables[relative])], env=self.env,
                              text=True, capture_output=True)

    def test_explicit_tools_and_sdk_are_used(self):
        for relative in self.executables:
            with self.subTest(crate=relative):
                self.log.unlink(missing_ok=True)
                result = self.run_script(relative)
                self.assertEqual(result.returncode, 0, result.stderr)
                calls = [json.loads(line) for line in self.log.read_text().splitlines()]
                self.assertEqual(calls[0][0], str(self.compiler))
                self.assertIn('--sysroot=' + self.env['AROS_SDK'], calls[0])
                self.assertEqual(calls[-1][0], str(self.archiver))
                if relative.endswith('libc-aros'):
                    for call in calls[:-1]:
                        source = call[call.index('-c') + 1]
                        self.assertIn('-O0' if '/vendor/jitterentropy/' in source else '-O2', call)
                for var in ('AROS_GCC', 'AROS_SDK', 'AROS_AR'):
                    self.assertIn('cargo:rerun-if-env-changed=' + var, result.stdout)

    def test_setup_records_explicit_archiver(self):
        repo = self.root / 'setup-copy'
        repo.mkdir()
        for name in ('setup.sh', 'x86_64-aros.json.in'):
            shutil.copy2(ROOT / name, repo / name)
        sdk = Path(self.env['AROS_SDK'])
        (sdk / 'include').mkdir(parents=True)
        (sdk / 'lib').mkdir()
        (sdk / 'lib/libcrt.a').touch()
        result = subprocess.run(['sh', str(repo / 'setup.sh')], env=self.env,
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        config = tomllib.loads((repo / 'aros-env.toml').read_text())
        self.assertEqual(config['env'].get('AROS_AR'), str(self.archiver))

    def test_setup_resolves_relative_archiver(self):
        self.env['AROS_AR'] = '../gcc-parent/custom-ar'
        self.test_setup_records_explicit_archiver()

    def test_missing_compiler_fails_before_tools_run(self):
        del self.env['AROS_GCC']
        for relative in self.executables:
            with self.subTest(crate=relative):
                result = self.run_script(relative)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('AROS_GCC', result.stderr)
                self.assertFalse(self.log.exists())


if __name__ == '__main__':
    unittest.main()
