"""Socket boundary and async consumers have deliberately different EOF contracts."""
from pathlib import Path
import json,socket,subprocess,sys,unittest

class FixtureEmptyTest(unittest.TestCase):
    def run_fixture(self,expected):
        held=[socket.socket(),socket.socket()]
        for sock in held:sock.bind(('127.0.0.1',0))
        ports=[sock.getsockname()[1] for sock in held]
        for sock in held:sock.close()
        script=Path(__file__).resolve().parents[1]/'ports/async-network/fixture.py'
        child=subprocess.Popen([sys.executable,str(script),'--port',str(ports[0]),'--refused-port',str(ports[1]),'--connections','2','--empty-connections',str(expected),'--deadline','2'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        try:
            ready=child.stdout.readline()
            self.assertIn('"ready"',ready,'fixture did not start with explicit empty-connection contract')
            for _ in range(2):
                with socket.create_connection(('127.0.0.1',ports[0]),timeout=1):pass
            out,err=child.communicate(timeout=3)
            return child.returncode,[json.loads(line) for line in out.splitlines()]
        finally:
            if child.poll() is None:child.terminate();child.wait(timeout=2)
            child.stdout.close();child.stderr.close()
    def test_exact_expected_empty_connections_are_accepted(self):
        code,rows=self.run_fixture(2)
        self.assertEqual(code,0);self.assertEqual(rows[-1],{'completed':2,'errors':[]})
    def test_async_contract_still_rejects_empty_connections(self):
        code,rows=self.run_fixture(0)
        self.assertNotEqual(code,0);self.assertTrue(rows[-1]['errors'])
