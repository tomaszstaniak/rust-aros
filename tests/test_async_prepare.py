import hashlib
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[1]

class ArchiveSafety(unittest.TestCase):
    def module(self):
        spec = importlib.util.spec_from_file_location('async_prepare', ROOT/'ports/async-network/prepare.py')
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        return module
    def archive(self, directory, name, content=b'source', link=False):
        archive = Path(directory)/'crate.tar.gz'
        with tarfile.open(archive,'w:gz') as tar:
            member=tarfile.TarInfo(name)
            if link:
                member.type=tarfile.SYMTYPE; member.linkname='/tmp/escape'
                tar.addfile(member)
            else:
                member.size=len(content); tar.addfile(member,io.BytesIO(content))
        return archive,hashlib.sha256(archive.read_bytes()).hexdigest()
    def test_checked_extract_and_existing_destination(self):
        module=self.module()
        with tempfile.TemporaryDirectory() as root:
            archive,digest=self.archive(root,'sample-1/src/lib.rs');dest=Path(root)/'out'
            module.extract_checked(archive,dest,'sample-1',digest)
            self.assertEqual((dest/'src/lib.rs').read_bytes(),b'source')
            with self.assertRaises(ValueError):module.extract_checked(archive,dest,'sample-1',digest)
            self.assertEqual((dest/'src/lib.rs').read_bytes(),b'source')
    def test_bad_digest_and_unsafe_members_never_create_output(self):
        module=self.module()
        for name,link in [('sample-1/../../escape',False),('/absolute',False),('other/src.rs',False),('sample-1/link',True)]:
            with self.subTest(name=name),tempfile.TemporaryDirectory() as root:
                archive,digest=self.archive(root,name,link=link);dest=Path(root)/'out'
                with self.assertRaises(ValueError):module.extract_checked(archive,dest,'sample-1',digest)
                self.assertFalse(dest.exists())
        with tempfile.TemporaryDirectory() as root:
            archive,_=self.archive(root,'sample-1/lib.rs');dest=Path(root)/'out'
            with self.assertRaises(ValueError):module.extract_checked(archive,dest,'sample-1','0'*64)
            self.assertFalse(dest.exists())
