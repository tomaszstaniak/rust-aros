#!/usr/bin/env python3
"""Build the geometry probe from the production function and real AROS headers.

AROS_GCC and AROS_SDK must identify the same ABI. No toolchain is modified.
"""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tests'))
from test_socket_boundary import function

production = function((ROOT / 'std-groundwork/libc-aros/csrc/compat.c').read_text(),
                      'static int rs_console_size(')
source = '''#include <stdio.h>
#include <stdint.h>
#include <limits.h>
#include <errno.h>
#include <string.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <proto/dos.h>
#include <dos/dosextens.h>
''' + production + '''
int probe_console_size(int fd, struct winsize *ws) { return rs_console_size(fd, ws); }
'''
with tempfile.TemporaryDirectory(prefix='aros-native-geometry-') as tmp:
    c = Path(tmp) / 'geometry.c'; c.write_text(source)
    sdk = os.environ['AROS_SDK']
    subprocess.run([os.environ['AROS_GCC'], f'--sysroot={sdk}', f'-I{sdk}/include',
                    '-O2', '-std=gnu99', '-Wall', '-Wextra', '-Werror', str(c),
                    str(ROOT / 'tests/native/console_geometry.c'), '-o', sys.argv[1],
                    '-lpthread'], check=True)
