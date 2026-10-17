import importlib.util
import socket
import threading
import unittest
from pathlib import Path

P = Path(__file__).resolve().parents[1] / 'ports/runtime/observe.py'
spec = importlib.util.spec_from_file_location('runtime_observe', P)
observe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(observe)

class RuntimeObservationTests(unittest.TestCase):
    def run_peer(self, messages, timeout=.2):
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0)); listener.listen()
            result = {}
            thread = threading.Thread(target=lambda: result.update(observe.collect(listener, 'run1', ['detach'], timeout)))
            thread.start()
            for message in messages:
                with socket.create_connection(listener.getsockname()) as peer:
                    peer.sendall((message + '\n').encode()); peer.shutdown(socket.SHUT_WR)
                    self.assertEqual(peer.recv(32), b'OK\n')
            thread.join(2)
            self.assertFalse(thread.is_alive())
            return result

    def test_shell_exit_receipt_passes(self):
        self.assertEqual(self.run_peer(['run1 detach start 0', 'run1 detach end 0'])['cases']['detach']['status'], 'passed')
    def test_nonzero_exit_is_failure(self):
        self.assertEqual(self.run_peer(['run1 detach start 0', 'run1 detach end 20'])['cases']['detach']['status'], 'failed')
    def test_pre_exit_marker_without_shell_receipt_times_out(self):
        self.assertEqual(self.run_peer(['run1 detach start 0'])['cases']['detach']['status'], 'timeout')
    def test_stale_identity_fails(self):
        self.assertTrue(self.run_peer(['old detach start 0'])['errors'])
    def test_end_before_start_fails(self):
        self.assertTrue(self.run_peer(['run1 detach end 0'])['errors'])

    def test_slow_bytes_cannot_renew_absolute_receipt_deadline(self):
        import time
        with socket.socket() as listener:
            listener.bind(('127.0.0.1',0));listener.listen()
            replies=[]
            def slow_peer():
                try:
                    with socket.create_connection(listener.getsockname(),timeout=1) as peer:
                        for byte in b'run1 detach start 0\n':
                            peer.sendall(bytes([byte]));time.sleep(.03)
                        replies.append(peer.recv(32))
                except OSError:
                    pass
            sender=threading.Thread(target=slow_peer)
            sender.start()
            started=time.monotonic()
            try:
                result=observe.collect(listener,'run1',['detach'],.1)
                elapsed=time.monotonic()-started
            finally:
                sender.join(2)
            self.assertFalse(sender.is_alive())
            self.assertLess(elapsed,.35,'per-recv timeout renewed beyond absolute host budget')
            self.assertNotIn(b'OK\n',replies,'receipt acknowledged after its absolute deadline')
            self.assertEqual(result['cases']['detach']['status'],'timeout')
