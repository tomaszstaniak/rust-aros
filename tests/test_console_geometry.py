"""Run the production geometry function against DOS packet boundaries."""
from pathlib import Path
import subprocess
import tempfile
import unittest
from test_socket_boundary import function

ROOT = Path(__file__).resolve().parents[1]

class ConsoleGeometry(unittest.TestCase):
    def test_geometry_snapshot_and_legacy_fallback(self):
        source = (ROOT / 'std-groundwork/libc-aros/csrc/compat.c').read_text()
        code = function(source, 'static int rs_console_size(')
        harness = r'''
#include <assert.h>
#include <errno.h>
#include <stdint.h>
#include <limits.h>
#include <string.h>
#include <stdlib.h>
#include <sys/ioctl.h>
typedef intptr_t BPTR;
typedef intptr_t SIPTR;
struct MsgPort { int unused; };
struct FileHandle { struct MsgPort *fh_Type; intptr_t fh_Arg1; };
#define BADDR(x) ((void *)(intptr_t)(x))
#define MKBADDR(x) ((intptr_t)(x))
#define ACTION_DISK_INFO 25
#define MEMF_PUBLIC 1
#define MEMF_CLEAR 2
#define ERROR_ACTION_NOT_KNOWN 209
#define DOSTRUE (-1)
#define DOSFALSE 0
static struct MsgPort port;
static struct FileHandle fh = { &port, 17 };
static int scenario, calls, interactive = 1, doserror;
static int __get_default_file(int fd, long *handle) {
    if (fd != 1) return -1;
    *handle = (long)(intptr_t)&fh; return 0;
}
static int IsInteractive(BPTR file) { (void)file; return interactive; }
static int IoErr(void) { return doserror; }
static intptr_t DoPkt(struct MsgPort *p, intptr_t action, intptr_t a1,
                     intptr_t a2, intptr_t a3, intptr_t a4, intptr_t a5) {
    calls++;
    assert(p == &port);
    assert(action != ACTION_DISK_INFO);
    assert(a1 == 17 && a5 == 0);
    uint32_t *v = (void *)a2;
    if (action == 0x434f4e47L) {
        assert(a3 == 24 && a4 == 1);
        if (scenario == 21) { doserror=ERROR_ACTION_NOT_KNOWN; return 1; }
        if (scenario >= 10) { memset(v,0xff,24); doserror = scenario == 11 ? 205 : ERROR_ACTION_NOT_KNOWN; return 0; }
        uint32_t reply[] = {24, 1, 80, 25, 640, 400};
        if (scenario == 1) reply[0] = 20;
        if (scenario == 2) reply[1] = 2;
        if (scenario == 3) reply[2] = 0;
        if (scenario == 4) reply[3] = 0;
        if (scenario >= 5 && scenario <= 8) reply[scenario - 3] = 65536;
        memcpy(v, reply, sizeof reply);
        return DOSTRUE;
    }
    assert(action == 0x58545901L && a3 == 64 && a4 == 0x58545931L);
    assert(v[0] == 1 && v[1] == 13);
    for (int i=2; i<16; i++) assert(v[i] == 0);
    if (scenario == 12) { doserror = ERROR_ACTION_NOT_KNOWN; return 0; }
    v[2] = 0x58545931L; v[4] = 8; v[8] = 25; v[9] = 80;
    if (scenario == 13) v[2] = 0;
    if (scenario == 14) v[3] = 1;
    if (scenario == 15) v[4] = 0;
    if (scenario == 16) v[8] = 0;
    if (scenario == 17) v[9] = 65536;
    if (scenario == 18) { v[4] |= 0x80000000; v[15] = 42; }
    if (scenario == 19) v[0] = 2;
    if (scenario == 20) v[1] = 4;
    return DOSTRUE;
}
''' + code + r'''
static void failure(int fd, int expected) {
    struct winsize ws, before;
    memset(&ws, 0xa5, sizeof ws); before = ws;
    assert(rs_console_size(fd, &ws) == -1);
    assert(errno == expected);
    assert(!memcmp(&ws, &before, sizeof ws));
}
int main(void) {
    struct winsize ws;
    assert(rs_console_size(1, &ws) == 0);
    assert(ws.ws_col == 80 && ws.ws_row == 25 && ws.ws_xpixel == 640 && ws.ws_ypixel == 400);
    assert(calls == 1);
    for (scenario=1; scenario<=8; scenario++) { calls=0; failure(1, scenario>=5 ? EOVERFLOW : ENOTTY); assert(calls==1); }
    scenario=10; calls=0;
    assert(rs_console_size(1,&ws)==0 && calls==2);
    assert(ws.ws_col==80 && ws.ws_row==25 && !ws.ws_xpixel && !ws.ws_ypixel);
    scenario=11; calls=0; failure(1,ENOTTY); assert(calls==1);
    for (scenario=12; scenario<=17; scenario++) failure(1,scenario==17 ? EOVERFLOW : ENOTTY);
    scenario=18; assert(rs_console_size(1,&ws)==0);
    for (scenario=19; scenario<=20; scenario++) failure(1,ENOTTY);
    scenario=21; calls=0; failure(1,ENOTTY); assert(calls==1);
    calls=0; failure(9,EBADF); assert(!calls);
    assert(rs_console_size(1,NULL)==-1 && errno==EFAULT && !calls);
    interactive=0; failure(1,ENOTTY); assert(!calls);
    interactive=1; fh.fh_Type=NULL; failure(1,ENOTTY); assert(!calls);
}
'''
        with tempfile.TemporaryDirectory(prefix='aros-geometry-') as directory:
            c = Path(directory) / 'test.c'; c.write_text(harness)
            binary = Path(directory) / 'test'
            result = subprocess.run(['cc','-std=c11','-Wall','-Wextra','-Werror','-fsanitize=address,undefined',str(c),'-o',str(binary)], capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            result = subprocess.run([str(binary)],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
