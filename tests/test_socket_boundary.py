"""Compile the production deferred-close function with a failing BSD boundary.

The compatibility unit depends on AROS library headers. Extract only this
function unchanged and supply the OS boundary; no replacement queue algorithm.
"""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

def function(source, signature):
    start = source.index(signature)
    body = source.index("{", start)
    depth = 1
    end = body + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]

class SocketBoundary(unittest.TestCase):
    def test_deferred_close_failure_retains_ownership_and_retries(self):
        source = (ROOT / "std-groundwork/libc-aros/csrc/compat.c").read_text()
        # Identify by name, allowing the implementation to change void to int.
        before = source.index("rs_socket_drain(void)")
        start = source.rfind("static ", 0, before)
        drain = function(source, source[start:before] + "rs_socket_drain(void)")
        harness = r"""
#include <assert.h>
#include <errno.h>
#define FD_SETSIZE 4
struct rs_socket_slot { unsigned char live, pending; int fdflags, status; };
static struct rs_socket_slot rs_socket_slots[FD_SETSIZE];
static int calls, fail = 1;
static void Forbid(void) {}
static void Permit(void) {}
static int Errno(void) { return EIO; }
static int CloseSocket(int fd) { assert(fd == 1); calls++; return fail ? -1 : 0; }
""" + drain + r"""
int main(void) {
    rs_socket_slots[1].live = rs_socket_slots[1].pending = 1;
    assert(rs_socket_drain() == -1 && errno == EIO);
    assert(calls == 1 && rs_socket_slots[1].live && rs_socket_slots[1].pending);
    fail = 0;
    assert(rs_socket_drain() == 0);
    assert(calls == 2 && !rs_socket_slots[1].live && !rs_socket_slots[1].pending);
    assert(rs_socket_drain() == 0 && calls == 2);
}
"""
        with tempfile.TemporaryDirectory(prefix="aros-socket-") as directory:
            c = Path(directory) / "test.c"; c.write_text(harness)
            binary = Path(directory) / "test"
            result = subprocess.run(["cc", "-Wall", "-Wextra", str(c), "-o", str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = subprocess.run([str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
