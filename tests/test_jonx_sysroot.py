"""Private source overlay must never mutate an upstream sysroot or checkout."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'tools/jonx-sysroot.py'


class JonxSysrootTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def module(self):
        spec = importlib.util.spec_from_file_location('jonx_sysroot', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_existing_destination_is_preserved(self):
        dest = self.root / 'dest'
        dest.mkdir()
        (dest / 'keep').write_text('original')
        with self.assertRaises(FileExistsError):
            self.module().create_sysroot(self.root / 'absent', self.root / 'absent', dest)
        self.assertEqual((dest / 'keep').read_text(), 'original')

    def test_overlay_sources_are_private_and_host_tools_use_explicit_sysroot(self):
        base, source, dest = (self.root / p for p in ('base', 'source', 'dest'))
        (base / 'bin').mkdir(parents=True)
        (base / 'bin/rustc').write_text('#!/bin/sh\nprintf "%s\\n" "$@"\n')
        (base / 'bin/rustc').chmod(0o755)
        (base / 'lib/rustlib/host').mkdir(parents=True)
        (source / 'library').mkdir(parents=True)
        (source / 'library/marker').write_text('source')
        (source / 'src').mkdir()
        self.module().create_sysroot(base, source, dest)
        copied = dest / 'lib/rustlib/src/rust/library/marker'
        copied.write_text('private')
        self.assertEqual((source / 'library/marker').read_text(), 'source')
        args = subprocess.check_output([dest / 'bin/rustc', '-V'], text=True).splitlines()
        self.assertEqual(args, ['--sysroot', str(dest), '-V'])
        self.assertTrue((dest / 'lib/rustlib/host').is_symlink())

    def test_wrong_revision_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'revision'):
            self.module().validate_source(Path(__file__).resolve().parents[1], '0' * 40)
