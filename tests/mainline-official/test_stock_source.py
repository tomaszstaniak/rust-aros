import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

SOURCE = Path(os.environ['AROS_OFFICIAL_SOURCE'])/'tools/crosstools/rust/unmodified'
spec=importlib.util.spec_from_file_location('stock_contract', SOURCE/'profiles.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)


class StockSourceTests(unittest.TestCase):
    def test_modified_missing_and_extra_source_files_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);f=root/'known.rs';f.write_text('stock');manifest={'known.rs':m.file_digest(f),'new.rs':None}
            m.validate_stock_source(root,manifest)
            f.write_text('changed')
            with self.assertRaises(ValueError):m.validate_stock_source(root,manifest)
            f.write_text('stock');extra=root/'unlisted.rs';extra.write_text('extra')
            with self.assertRaises(ValueError):m.validate_stock_source(root,manifest)
            extra.unlink();f.unlink()
            with self.assertRaises(ValueError):m.validate_stock_source(root,manifest)

    def test_generated_library_hash_changes_sdk_generation(self):
        before={'/native/libstd.rlib':'0'*64};after={'/native/libstd.rlib':'1'*64}
        self.assertNotEqual(m.generation(before,'x86_64-unknown-aros',[]),m.generation(after,'x86_64-unknown-aros',[]))
