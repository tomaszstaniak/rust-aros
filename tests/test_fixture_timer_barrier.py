"""Timer/network concurrency must use protocol ordering, not two clock domains."""
from pathlib import Path
import json,socket,subprocess,sys,unittest
class TimerBarrierTest(unittest.TestCase):
    def test_response_requires_timer_completion_control_connection(self):
        held=[socket.socket(),socket.socket()]
        for s in held:s.bind(('127.0.0.1',0))
        ports=[s.getsockname()[1] for s in held]
        for s in held:s.close()
        script=Path(__file__).resolve().parents[1]/'ports/async-network/fixture.py'
        child=subprocess.Popen([sys.executable,str(script),'--port',str(ports[0]),'--refused-port',str(ports[1]),'--connections','2','--deadline','2'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        try:
            self.assertIn('"ready"',child.stdout.readline())
            with socket.create_connection(('127.0.0.1',ports[0]),timeout=1) as waiting:
                waiting.sendall(b'timer-race\n');waiting.settimeout(.05)
                with self.assertRaises(TimeoutError):waiting.recv(1)
                with socket.create_connection(('127.0.0.1',ports[0]),timeout=1) as control:
                    control.sendall(b'timer-done\n');self.assertEqual(control.recv(1),b'R')
                waiting.settimeout(1);self.assertEqual(waiting.recv(1),b'R')
            out,err=child.communicate(timeout=3)
            self.assertEqual(child.returncode,0,err);self.assertEqual(json.loads(out.splitlines()[-1]),{'completed':2,'errors':[]})
        finally:
            if child.poll() is None:child.terminate();child.wait(timeout=2)
            child.stdout.close();child.stderr.close()
