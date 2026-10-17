"""A combined run must control each child fixture's actual accept deadline."""
from pathlib import Path
import socket,subprocess,sys,time,unittest

class FixtureDeadlineTest(unittest.TestCase):
    def test_configured_accept_deadline_is_enforced(self):
        ports=[]
        for _ in range(2):
            with socket.socket() as sock:
                sock.bind(('127.0.0.1',0));ports.append(sock.getsockname()[1])
        script=Path(__file__).resolve().parents[1]/'ports/async-network/fixture.py'
        start=time.monotonic()
        result=subprocess.run([sys.executable,str(script),'--port',str(ports[0]),'--refused-port',str(ports[1]),'--connections','1','--deadline','0.05'],capture_output=True,text=True,timeout=2)
        self.assertIn('"ready"',result.stdout,'fixture did not enter the timed accept phase')
        self.assertNotEqual(result.returncode,0,'missing guest must fail acceptance')
        self.assertLess(time.monotonic()-start,1.5,'configured deadline was ignored')
        self.assertIn('timed out',result.stderr)
