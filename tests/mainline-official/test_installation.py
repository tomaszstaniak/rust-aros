"""Fail closed before using an installed official SDK."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

SOURCE = Path(os.environ['AROS_OFFICIAL_SOURCE']) / 'tools/crosstools/rust/unmodified'
spec = importlib.util.spec_from_file_location('profiles_contract', SOURCE / 'profiles.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)


class InstallationTests(unittest.TestCase):
    def fixture(self, root):
        profile = m.select('x86_64-unknown-aros')
        names = ['share/x86_64-unknown-aros.json', 'cargo-config.toml'] + ['bin/' + name for name in
                 ('rustc', 'cargo', 'rustdoc', 'x86_64-aros-cc')]
        for name in names:
            p = root / name; p.parent.mkdir(parents=True, exist_ok=True); p.write_text(name)
        external = root.parent / 'input'; external.write_text('input')
        inputs = {str(external): m.file_digest(external)}
        manifest = dict(schema=1, profile=profile, inputs=inputs, cc_args=[], compiler_commit='pinned',
                        generation=m.generation(inputs, profile['triple'], []),
                        installed={name: m.file_digest(root / name) for name in names})
        return manifest

    def run_case(self, change=None, mutate=None):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'prefix'; root.mkdir(); data = self.fixture(root)
            if change: change(data)
            if mutate: mutate(root)
            (root / 'aros-official-rust.json').write_text(json.dumps(data))
            return m.validate_installation(root, 'pinned')

    def test_complete_manifest(self):
        self.assertEqual(self.run_case()['schema'], 1)

    def test_bad_schema_or_generation(self):
        for field, value in [('schema', 99), ('generation', '0' * 64), ('compiler_commit', 'other')]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.run_case(lambda data: data.update({field: value}))

    def test_empty_missing_extra_and_bad_hash(self):
        for change in (lambda d: d.update(installed={}), lambda d: d['installed'].pop('bin/rustc'),
                       lambda d: d['installed'].update({'../escape': '0' * 64}),
                       lambda d: d['installed'].update({'bin/rustc': '0' * 64})):
            with self.assertRaises(ValueError): self.run_case(change)

    def test_modified_input_or_wrapper(self):
        with self.assertRaises(ValueError):
            self.run_case(mutate=lambda root: (root / 'bin/rustc').write_text('changed'))
        with self.assertRaises(ValueError):
            self.run_case(mutate=lambda root: (root.parent / 'input').write_text('changed'))
