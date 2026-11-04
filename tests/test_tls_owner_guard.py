"""Foreign construction must reject before reading mutable owner-only TLS globals."""
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

class TlsOwnerGuardTest(unittest.TestCase):
    def test_foreign_constructor_does_not_read_owner_globals(self):
        source=(Path(__file__).resolve().parents[1]/'ports/async-network/tls_adapter.c').read_text()
        function=source.split('struct rs_tls *rs_tls_new(',1)[1]
        condition=re.search(r'if \(([^\n]*rs_selector_owner[^\n]*)\) \{',function).group(1)
        program='''#include <stddef.h>
static int reads;
static int read_live(void) { ++reads; return 1; }
#define live read_live()
static void *crypto_owner;
static void *FindTask(void *p) { return (void *)1; }
static int rs_selector_owner(void) { return -1; }
int main(void) { if (CONDITION) { return reads ? 7 : 0; } return 8; }
'''.replace('CONDITION',condition)
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory); (path/'guard.c').write_text(program)
            subprocess.run(['cc','-std=c99',str(path/'guard.c'),'-o',str(path/'guard')],check=True,capture_output=True)
            result=subprocess.run([str(path/'guard')],capture_output=True)
            self.assertEqual(result.returncode,0,'foreign constructor read mutable owner globals before rejecting')
