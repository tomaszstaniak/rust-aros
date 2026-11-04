import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

SOURCE = Path(os.environ['AROS_OFFICIAL_SOURCE']) / 'tools/crosstools/rust/unmodified'
spec = importlib.util.spec_from_file_location('native_identity', SOURCE / 'profiles.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)


class NativeIdentityTests(unittest.TestCase):
    def test_sdk_and_tools_changes_invalidate_reused_build(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); sdk = root/'sdk'; tools = root/'tools'
            for name in ('include/header.h', 'lib/startup.o'):
                p = sdk/name; p.parent.mkdir(parents=True, exist_ok=True); p.write_text(name)
            for name in ('llvm-ar', 'llvm-nm', 'llvm-objcopy', 'ld.lld'):
                p = tools/'bin'/name; p.parent.mkdir(parents=True, exist_ok=True); p.write_text(name)
            cc = root/'cc'; cc.write_text('cc'); collect = root/'collect'; collect.write_text('collect')
            initial = m.native_inputs(sdk, tools, collect, cc)
            for path in (sdk/'include/header.h', sdk/'lib/startup.o', cc, tools/'bin/ld.lld'):
                original = path.read_text(); path.write_text('changed')
                self.assertNotEqual(initial, m.native_inputs(sdk, tools, collect, cc))
                path.write_text(original)
            (sdk/'include/new.h').write_text('new')
            self.assertNotEqual(initial, m.native_inputs(sdk, tools, collect, cc))

    def test_requested_imports_must_equal_retained_imports(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'imports.a'; path.write_bytes(b'new')
            expected = {'libaros_positional.a': m.file_digest(path)}
            m.validate_imports(expected, path)
            for retained, requested in (({}, path), (expected, None),
                                        ({'libaros_positional.a': '0' * 64}, path)):
                with self.assertRaises(ValueError): m.validate_imports(retained, requested)
