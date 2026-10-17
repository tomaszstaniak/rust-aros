from pathlib import Path
import subprocess
import tempfile
import unittest

class RuntimeReceiptCodeTests(unittest.TestCase):
    def test_file_codes_and_invalid_literal_are_distinguished(self):
        root=Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as directory:
            d=Path(directory);source=d/'test.c'
            source.write_text('#include "receipt_code.h"\nint main(int argc,char **argv){char code[32];if(argc!=2 || read_exit_code(argv[1],code)) return 20;puts(code);return 0;}\n')
            subprocess.run(['cc','-I',str(root/'ports/runtime'),str(source),'-o',str(d/'probe')],check=True,capture_output=True)
            for text,rc,want in [('0\n',0,'0'),('20\n',0,'20'),('$probe_rc\n',20,''),('0 trailing\n',20,''),('',20,''),('-1\n',20,'')]:
                p=d/'code';p.write_text(text)
                actual=subprocess.run([str(d/'probe'),str(p)],capture_output=True,text=True)
                self.assertEqual((actual.returncode,actual.stdout.strip()),(rc,want))
            self.assertEqual(subprocess.run([str(d/'probe'),str(d/'missing')]).returncode,20)
