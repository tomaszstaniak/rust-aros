from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
HARNESS = r'''
#include <assert.h>
#include <errno.h>
#include <stddef.h>
#include <string.h>
int getentropy(void *, size_t);
static int calls, mode;
int rs_jent_poll(void *unused, unsigned char *out, size_t len, size_t *got) {
    (void)unused;
    calls++;
    memset(out, 0x5a, len);
    *got = mode == 2 ? len - 1 : len;
    return mode == 1 ? -60 : 0;
}
int main(void) {
    unsigned char buf[257];
    assert(getentropy(NULL, 0) == 0 && calls == 0);
    errno = 0;
    assert(getentropy(buf, sizeof buf) == -1 && errno == EIO && calls == 0);
    assert(getentropy(NULL, 1) == -1 && errno == EFAULT && calls == 0);
    assert(getentropy(buf, 256) == 0 && calls == 1);
    for (int i = 0; i < 256; i++) assert(buf[i] == 0x5a);
    for (mode = 1; mode <= 2; mode++) {
        memset(buf, 0xcc, sizeof buf);
        errno = 0;
        assert(getentropy(buf, 256) == -1 && errno == EIO);
        for (int i = 0; i < 256; i++) assert(buf[i] == 0);
        assert(buf[256] == 0xcc);
    }
    assert(calls == 3);
    mode = 0;
    assert(getentropy(buf, 1) == 0 && calls == 4 && buf[0] == 0x5a);
    return 0;
}
'''


class EntropyContract(unittest.TestCase):
    def test_posix_contract_and_source_failures(self):
        with tempfile.TemporaryDirectory(prefix='aros-entropy-') as directory:
            root = Path(directory)
            harness = root / 'harness.c'
            harness.write_text(HARNESS)
            binary = root / 'check'
            result = subprocess.run(['cc', '-Wall', '-Wextra', '-Werror',
                                     '-Dgetentropy=rs_test_getentropy', str(harness),
                                     str(ROOT / 'std-groundwork/libc-aros/csrc/entropy.c'),
                                     '-o', str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = subprocess.run([str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()
