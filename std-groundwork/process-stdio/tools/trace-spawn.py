#!/usr/bin/env python3
"""Instrument only a disposable patched std src/sys/process/unix/unix.rs.

Links solely with this fixture's rs_probe_trace. Never use the resulting
sysroot as a general toolchain. Trace calls can change timing and stack layout.
"""
from pathlib import Path
import sys
p = Path(sys.argv[1])
s = p.read_text()
first_exec = s.index('    unsafe fn do_exec(')
boundary = s.index('    unsafe fn do_exec(', first_exec + 1)
suffix = s[boundary:]
s = s[:boundary]
assert 'fn rs_probe_trace' not in s
s = 'unsafe extern "C" { fn rs_probe_trace(stage: i32, value: isize); }\n' + s
# Restrict edits to the fork/exec fallback, not posix_spawn.
changes = [
    ('        let pid = unsafe { self.do_fork()? };', '        unsafe { rs_probe_trace(0, 0); }\n        let pid = unsafe { self.do_fork()? };\n        unsafe { rs_probe_trace(if pid == 0 { 1 } else { 11 }, pid as isize); }'),
    ('            let Err(err) = unsafe { self.do_exec(theirs, envp.as_ref()) };', '            let Err(err) = unsafe { self.do_exec(theirs, envp.as_ref()) };\n            unsafe { rs_probe_trace(12, err.raw_os_error().unwrap_or(-999) as isize); }'),
    ('        if let Some(fd) = stdio.stdin.fd() {', '        unsafe { rs_probe_trace(2, 0); }\n        if let Some(fd) = stdio.stdin.fd() {'),
    ('        if let Some(fd) = stdio.stdout.fd() {', '        unsafe { rs_probe_trace(3, 0); }\n        if let Some(fd) = stdio.stdout.fd() {'),
    ('        if let Some(fd) = stdio.stderr.fd() {', '        unsafe { rs_probe_trace(4, 0); }\n        if let Some(fd) = stdio.stderr.fd() {'),
    ('        #[cfg(not(target_os = "l4re"))]\n        {', '        unsafe { rs_probe_trace(5, 0); }\n        #[cfg(not(target_os = "l4re"))]\n        {'),
    ('        // emscripten has no signal support.', '        unsafe { rs_probe_trace(6, 0); }\n        // emscripten has no signal support.'),
    ('                    let ret = sys::signal(libc::SIGPIPE, libc::SIG_DFL);', '                    let ret = sys::signal(libc::SIGPIPE, libc::SIG_DFL);\n                    unsafe { rs_probe_trace(7, ret as isize); }'),
    ('        for callback in self.get_closures().iter_mut() {', '        unsafe { rs_probe_trace(8, 0); }\n        for callback in self.get_closures().iter_mut() {'),
    ('        libc::execvp(self.get_program_cstr().as_ptr(), self.get_argv().as_ptr());', '        unsafe { rs_probe_trace(9, 0); }\n        libc::execvp(self.get_program_cstr().as_ptr(), self.get_argv().as_ptr());\n        unsafe { rs_probe_trace(10, crate::sys::io::errno() as isize); }'),
]
for before, after in changes:
    assert s.count(before) == 1, before
    s = s.replace(before, after)
p.write_text(s + suffix)
