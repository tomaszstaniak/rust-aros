"""Profile admission tests run against the privately prepared Jonx source."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

SOURCE = Path(os.environ['AROS_OFFICIAL_SOURCE']) / 'tools/crosstools/rust/unmodified'


class ProfilesTests(unittest.TestCase):
    def module(self):
        spec = importlib.util.spec_from_file_location('official_profiles', SOURCE/'profiles.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_default_arm_unchanged(self):
        self.assertEqual(self.module().select('aarch64-unknown-aros')['machine'],183)

    def test_only_explicit_x86_v1_admitted(self):
        profile=self.module().select('x86_64-unknown-aros')
        self.assertEqual((profile['machine'],profile['abi'],profile['runtime']),(62,1,'UNTESTED'))
        for name in ('x86_64-v11','i686-unknown-aros','powerpc-unknown-aros'):
            with self.assertRaises(ValueError):self.module().select(name)

    def test_wrong_cpu_sdk_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'lib').mkdir();(root/'include/aros').mkdir(parents=True)
            header=bytearray(64);header[16:18]=(1).to_bytes(2,'little');header[:6]=bytes.fromhex('7f454c460201');header[18:20]=(183).to_bytes(2,'little')
            (root/'lib/startup.o').write_bytes(header)
            (root/'include/aros/config.h').write_text('#define AROS_DOS_PACKETS 1\n')
            with self.assertRaises(ValueError):self.module().validate_sdk(root,self.module().select('x86_64-unknown-aros'))

    def test_v11_sdk_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'lib').mkdir();header=bytearray(64);header[16:18]=(1).to_bytes(2,'little');header[:6]=bytes.fromhex('7f454c460201');header[18:20]=(62).to_bytes(2,'little');(root/'lib/startup.o').write_bytes(header)
            with self.assertRaises(ValueError):self.module().validate_sdk(root,self.module().select('x86_64-unknown-aros'))

    def test_driver_identity_includes_flags_and_sdk(self):
        m = self.module()
        profile = m.select('x86_64-unknown-aros')
        actual = m.c_driver(profile, Path('/sdk'), Path('/tools'), Path('/cc'), ['-mno-red-zone'])
        self.assertEqual(actual, ['/cc', '-mno-red-zone', '--sysroot=/sdk'])
        self.assertNotEqual(actual, m.c_driver(profile, Path('/sdk'), Path('/tools'), Path('/cc'), []))
        self.assertNotEqual(actual, m.c_driver(profile, Path('/other'), Path('/tools'), Path('/cc'), ['-mno-red-zone']))

    def test_native_v1_sdk_is_admitted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'lib').mkdir(); (root / 'include/aros').mkdir(parents=True)
            header = bytearray(64); header[16:18] = (1).to_bytes(2, 'little'); header[:6] = bytes.fromhex('7f454c460201')
            header[18:20] = (62).to_bytes(2, 'little')
            (root / 'lib/startup.o').write_bytes(header)
            (root / 'include/aros/config.h').write_text('#define AROS_DOS_PACKETS                1\n')
            (root / 'include/pthread.h').write_text('struct pthread_condattr { int pshared; clockid_t cond_clock; };\n')
            (root / 'include/sys').mkdir()
            (root / 'include/sys/cdefs.h').write_text('#define __POSIX_VISIBLE 200809\n')
            self.module().validate_sdk(root, self.module().select('x86_64-unknown-aros'))

    def test_large_model_got_relocations_are_rejected(self):
        m = self.module()
        profile = m.select('x86_64-unknown-aros')
        m.validate_relocations(profile, {1: 10, 2: 2, 24: 5})
        for number in (3, 9, 19, 20, 22, 25, 27, 28, 29, 30, 31, 41, 42, 999):
            with self.subTest(number=number), self.assertRaises(ValueError):
                m.validate_relocations(profile, {number: 1})

    def test_old_mainline_pthread_headers_are_not_current_profile(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); (root / 'lib').mkdir(); (root / 'include/aros').mkdir(parents=True)
            header = bytearray(64); header[:6] = bytes.fromhex('7f454c460201')
            header[16:18] = (1).to_bytes(2, 'little'); header[18:20] = (62).to_bytes(2, 'little')
            (root / 'lib/startup.o').write_bytes(header)
            (root / 'include/aros/config.h').write_text('#define AROS_DOS_PACKETS                1\n')
            (root / 'include/pthread.h').write_text('struct pthread_condattr { int pshared; };\n')
            with self.assertRaises(ValueError):
                self.module().validate_sdk(root, self.module().select('x86_64-unknown-aros'))
