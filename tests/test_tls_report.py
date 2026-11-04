"""Acceptance must reject stale or incomplete native evidence, not just find PASS."""
import importlib.util
from pathlib import Path
import unittest
import tempfile
import json
import hashlib

PORT=Path(__file__).resolve().parents[1]/'ports/async-network'
class TlsReportTests(unittest.TestCase):
    def checker(self):
        path=PORT/'tls-check.py'
        self.assertTrue(path.exists(),'TLS report checker is missing')
        spec=importlib.util.spec_from_file_location('tls_check',path)
        mod=importlib.util.module_from_spec(spec); spec.loader.exec_module(mod)
        return mod.validate_reports
    def evidence(self):
        run='unique-build-id'
        normal='START TLS Mbed3.6.7 jitterentropy3.7 x86_64-v11 run='+run+'\n'
        normal+='\n'.join(['PASS foreign TLS construction rejected; owner context preserved','PASS verified HTTPS with authenticated EOF',*[f'PASS certificate rejection port={p} flag={f} detail' for p,f in [(18771,4),(18772,1),(18771,8)]],'PASS pending handshake cancellation, live drop, and subsequent connection','PASS cleanup after rejection and concurrent TLS context lifetime','PASS raw transport EOF rejected','PASS HTTPS acceptance run='+run])+'\n'
        entropy='START TLS Mbed3.6.7 jitterentropy3.7 x86_64-v11 run='+run+'\nPASS forced entropy failure before connect detail\nPASS HTTPS acceptance run='+run+'\n'
        shell=f'START {run}\n0\n0\nDONE {run}\n'
        records=[dict(run_id=run,kind='ready')]
        for kind,count,port in [('verified-http',4,18771),('certificate-alert',2,18771),('certificate-alert',1,18772),('truncated',1,18774),('cancelled-handshake',1,18775),('live-drop',1,18776)]:
            records.extend(dict(run_id=run,kind=kind,port=port) for _ in range(count))
        records.append(dict(run_id=run,kind='completed',accepted=10,entropy_connections=0,errors=[]))
        return [run,normal,entropy,shell,records]
    def test_accepts_complete_matching_run(self): self.checker()(*self.evidence())
    def test_rejects_stale_guest_report(self):
        values=self.evidence(); values[1]=values[1].replace('unique-build-id','old-build-id')
        with self.assertRaises(ValueError): self.checker()(*values)
    def test_rejects_missing_certificate_case(self):
        values=self.evidence();values[1]='\n'.join(l for l in values[1].splitlines() if 'flag=1 ' not in l)
        with self.assertRaises(ValueError): self.checker()(*values)
    def test_rejects_entropy_connection(self):
        values=self.evidence();values[4][-1]['entropy_connections']=1
        with self.assertRaises(ValueError): self.checker()(*values)

class TlsSourceIntegrityTests(unittest.TestCase):
    def test_changed_compiled_dependency_and_header_are_rejected(self):
        spec=importlib.util.spec_from_file_location('tls_check',PORT/'tls-check.py')
        mod=importlib.util.module_from_spec(spec); spec.loader.exec_module(mod)
        for name in ['async-io/src/driver_aros.rs','libc-source/csrc/compat.c','include/mbedtls_aros_config.h']:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                build=Path(directory); source=build/name; source.parent.mkdir(parents=True); source.write_text('original input')
                expected=hashlib.sha256(source.read_bytes()).hexdigest()
                (build/'tls-build.json').write_text(json.dumps(dict(status='built-not-run',artifacts={},sources={name:expected},adapter={})))
                source.write_text('changed input')
                with self.assertRaisesRegex(ValueError,'build source changed'):
                    mod.check(build,build,'report',build/'fixture.log')
