"""Private detach objects must match their SDK, CPU profile and receipt."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('runtime_common', ROOT/'ports/runtime/build-common.py')
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class PrivatePthreadObjectsTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.sdk = self.root/'sdk'
        (self.sdk/'lib').mkdir(parents=True)
        (self.sdk/'lib/libpthread.a').write_bytes(b'sdk')
        self.objects = self.root/'objects'
        self.objects.mkdir()
        hashes = {}
        for name in ('pthread.o','pthread_create.o'):
            data=name.encode();(self.objects/name).write_bytes(data)
            hashes[name]=hashlib.sha256(data).hexdigest()
        self.manifest=dict(profile='aarch64-v1-raspi',variant='fixed',objects=hashes,
                           sdk_pthread_sha256=hashlib.sha256(b'sdk').hexdigest(),
                           fix_sha256=hashlib.sha256((ROOT/'ports/runtime/patches/pthread.patch').read_bytes()).hexdigest())
        self.save()

    def save(self):
        (self.objects/'build.json').write_text(json.dumps(self.manifest))

    def validate(self):
        return builder.verified_pthread_objects(self.objects,self.sdk,'aarch64-v1-raspi')

    def test_valid(self):
        self.assertEqual(len(self.validate()),2)

    def test_wrong_cpu(self):
        self.manifest['profile']='x86_64-v11';self.save()
        with self.assertRaises(ValueError):self.validate()

    def test_sdk_changed(self):
        (self.sdk/'lib/libpthread.a').write_bytes(b'new-sdk')
        with self.assertRaises(ValueError):self.validate()

    def test_object_changed(self):
        (self.objects/'pthread.o').write_bytes(b'changed')
        with self.assertRaises(ValueError):self.validate()

    def test_missing_inventory_entry(self):
        del self.manifest['objects']['pthread_create.o'];self.save()
        with self.assertRaises(ValueError):self.validate()

    def test_old_patch(self):
        self.manifest['fix_sha256']='0'*64;self.save()
        with self.assertRaises(ValueError):self.validate()

    def test_baseline_is_not_a_correction(self):
        self.manifest['variant']='baseline';self.save()
        with self.assertRaises(ValueError):self.validate()
