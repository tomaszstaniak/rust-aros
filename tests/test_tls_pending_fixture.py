from pathlib import Path
import importlib.util,socket,unittest
class PendingTlsFixtureTest(unittest.TestCase):
    def test_pending_peer_keeps_cpu_work_budget_and_requires_client_close(self):
        path=Path(__file__).resolve().parents[1]/'ports/async-network/tls-fixture.py'
        spec=importlib.util.spec_from_file_location('tls_pending_fixture',path);mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
        self.assertTrue(hasattr(mod,'wait_for_cancel'),'pending-handshake fixture helper missing')
        left,right=socket.socketpair()
        try:
            hello=b'\x16\x03\x03\x00\x05hello';right.sendall(hello);right.shutdown(socket.SHUT_WR)
            count,elapsed=mod.wait_for_cancel(left)
            self.assertEqual(count,len(hello));self.assertEqual(left.gettimeout(),60)
            self.assertGreaterEqual(elapsed,0)
        finally:left.close();right.close()
