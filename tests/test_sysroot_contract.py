import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('contract', ROOT / 'tools/sysroot_contract.py')
contract = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contract)


class SysrootContractTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for name in ('library', 'src'):
            path = self.root / contract.SOURCE / name
            path.mkdir(parents=True)
            (path / 'source.rs').write_text('original')
        contract.record(self.root)

    def validate(self):
        return contract.validate(self.root, 'aarch64-v1-raspi')

    def mutate(self, change):
        path = self.root / 'aros-common.json'
        data = json.loads(path.read_text())
        change(data)
        path.write_text(json.dumps(data))
        with self.assertRaises(ValueError):
            self.validate()

    def test_valid(self):
        self.validate()

    def test_missing(self):
        (self.root / 'aros-common.json').unlink()
        with self.assertRaises(ValueError):
            self.validate()

    def test_invalid_schema(self):
        self.mutate(lambda data: data.update(schema=0))

    def test_wrong_compiler(self):
        self.mutate(lambda data: data.update(compiler='old'))

    def test_wrong_pal(self):
        self.mutate(lambda data: data.update(jonx='0' * 40))

    def test_wrong_profile(self):
        self.mutate(lambda data: data.update(backends=['x86_64-v11']))

    def test_stale_adaptations(self):
        self.mutate(lambda data: data.update(adaptations={}))

    def test_incomplete_sources(self):
        self.mutate(lambda data: data.update(sources={}))

    def test_modified_source(self):
        (self.root / contract.SOURCE / 'library/source.rs').write_text('changed')
        with self.assertRaisesRegex(ValueError, 'modified'):
            self.validate()

    def test_removed_source(self):
        (self.root / contract.SOURCE / 'src/source.rs').unlink()
        with self.assertRaises(ValueError):
            self.validate()

    def test_added_source(self):
        (self.root / contract.SOURCE / 'library/extra.rs').write_text('added')
        with self.assertRaises(ValueError):
            self.validate()

    def test_builder_rejects_unverified_manifest_before_compiler(self):
        self.mutate(lambda data: data.update(compiler='old'))
        args = ['python3', str(ROOT / 'tools/build-acceptance.py'),
                '--profile', 'aarch64-v1-raspi']
        for name in ('sysroot', 'sdk', 'cc', 'ar', 'nm', 'out'):
            args += ['--' + name, str(self.root if name == 'sysroot' else self.root / name)]
        result = subprocess.run(args, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('sysroot compiler', result.stderr)
        self.assertNotIn('Traceback', result.stderr)
        self.assertFalse((self.root / 'out').exists())
