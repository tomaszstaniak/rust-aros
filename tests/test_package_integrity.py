import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
NAMES = tuple(f'aros-{kind}-acceptance' for kind in ('c', 'nostd', 'std'))


class PackageIntegrityTests(unittest.TestCase):
    def package(self, change):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build = root / 'build'
            build.mkdir()
            artifacts = {}
            for name in NAMES:
                data = name.encode()
                (build / name).write_bytes(data)
                artifacts[name] = hashlib.sha256(data).hexdigest()
            change(artifacts)
            (build / 'build.json').write_text(json.dumps(dict(
                artifacts=artifacts, profile='test', compiler='test')))
            bundle = root / 'bundle'
            result = subprocess.run([
                'python3', str(ROOT / 'tools/package-acceptance.py'),
                str(build), str(bundle), '--programs', 'RAM:test',
                '--results', 'RAM:', '--run-id', 'unique'], capture_output=True)
            if result.returncode == 0:
                for name, kind in zip(NAMES, ('c', 'nostd', 'std')):
                    self.assertEqual((bundle / kind).read_bytes(), (build / name).read_bytes())
                self.assertTrue((bundle / 'run').is_file())
            else:
                self.assertFalse(bundle.exists(), result.stderr.decode())
            return result.returncode

    def test_empty_inventory(self):
        self.assertNotEqual(self.package(lambda entries: entries.clear()), 0)

    def test_missing_entry(self):
        self.assertNotEqual(self.package(lambda entries: entries.pop(NAMES[0])), 0)

    def test_extra_entry(self):
        self.assertNotEqual(self.package(lambda entries: entries.update(extra='0' * 64)), 0)

    def test_bad_hash(self):
        self.assertNotEqual(self.package(lambda entries: entries.update({NAMES[1]: '0' * 64})), 0)

    def test_valid_inventory(self):
        self.assertEqual(self.package(lambda entries: None), 0)
