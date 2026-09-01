#!/usr/bin/env python3
"""Teach a copy of rust-src's `std` about target_os = "aros".

Usage: patch-std.py <path to library/std/src>

AROS is a unix-family target for std's purposes: real pthreads, real POSIX
file I/O. What it lacks (fork, mmap, dynamic loading) std can live without.
Every edit below adds "aros" next to an existing small target, usually haiku,
so the diff stays reviewable against upstream.
"""
import os, re, sys

STD = sys.argv[1]

def edit(rel, fn):
    p = os.path.join(STD, rel)
    s = open(p).read()
    t = fn(s)
    if t == s:
        print(f"  (no change) {rel}")
    else:
        open(p, "w").write(t)
        print(f"  patched {rel}")

def add_after(s, anchor, extra, count=1):
    assert anchor in s, f"anchor not found: {anchor!r}"
    return s.replace(anchor, anchor + extra, count)

print("patching std at", STD)
marker = os.path.join(STD, "os/aros/.patched")
if os.path.exists(marker):
    print("  already patched; restore a clean rust-src copy to re-apply"); sys.exit(0)

# 1. os::aros module: raw types and MetadataExt, modelled on os::haiku.
os.makedirs(os.path.join(STD, "os/aros"), exist_ok=True)
for f in ("mod.rs", "raw.rs", "fs.rs"):
    src = open(os.path.join(STD, "os/haiku", f)).read().replace("haiku", "aros").replace("Haiku", "AROS")
    # haiku's fs.rs carries per-item stability for st_crtime; strip duplicate attrs
    src = re.sub(r'#\[stable\(feature = "metadata_ext2", since = "[0-9.]+"\)\]\n(\s*#\[stable)', r'\1', src)
    open(os.path.join(STD, "os/aros", f), "w").write(src)
print("  created os/aros/{mod,raw,fs}.rs from os/haiku")

edit("os/mod.rs", lambda s: add_after(s, '#[cfg(target_os = "haiku")]\npub mod haiku;', '\n#[cfg(target_os = "aros")]\npub mod aros;'))
edit("os/unix/mod.rs", lambda s: add_after(s, '    #[cfg(target_os = "haiku")]\n    pub use crate::os::haiku::*;', '\n    #[cfg(target_os = "aros")]\n    pub use crate::os::aros::*;'))

# 2. args: AROS' C startup hands argc/argv to main; use the "really_init"
#    path that stores them from the main() entry, like haiku does.
edit("sys/args/unix.rs", lambda s: s.replace('    target_os = "haiku",\n', '    target_os = "haiku",\n    target_os = "aros",\n', 1))

# 3. random: no arc4random on AROS; use getentropy from the SDK's C library.
edit("sys/random/mod.rs", lambda s: add_after(s, '    target_os = "emscripten" => {\n        mod getentropy;\n        pub use getentropy::fill_bytes;\n    }',
     '\n    target_os = "aros" => {\n        mod getentropy;\n        pub use getentropy::fill_bytes;\n    }'))

# 4. current_exe: nothing like /proc; fall back to the argv[0]-based search
#    that redox/haiku use.
edit("sys/paths/unix.rs", lambda s: add_after(s, '#[cfg(target_os = "rtems")]\npub fn current_exe() -> io::Result<PathBuf> {\n    crate::fs::read_to_string("sys:exe").map(PathBuf::from)\n}',
     '\n\n#[cfg(target_os = "aros")]\npub fn current_exe() -> io::Result<PathBuf> {\n    // No /proc on AROS; the program lives where it was started from.\n    crate::env::args_os().next().map(PathBuf::from)\n        .ok_or_else(|| io::const_error!(io::ErrorKind::NotFound, "no argv[0]"))\n}'))

# 5. thread names: no pthread_setname_np semantics worth relying on; opt out
#    like l4re/aix so set_name becomes a no-op.
edit("sys/thread/mod.rs", lambda s: s.replace('            target_os = "aix",\n            target_os = "wasi",\n        )))]\n        pub use unix::set_name;',
     '            target_os = "aix",\n            target_os = "aros",\n            target_os = "wasi",\n        )))]\n        pub use unix::set_name;', 1))

# 6. stat times: handled in the libc crate, whose `stat` exposes st_atime/st_atime_nsec.

# 7. os::aros::fs came from haiku, which has a creation time; AROS' stat has none.
def drop_crtime(s):
    # remove each st_crtime* item together with the attribute line above it
    s = re.sub(r'\n\s*#\[stable\([^\n]*\)\]\n\s*fn st_crtime(_nsec)?\(&self\) -> i64;', '', s)
    s = re.sub(r'\n\s*fn st_crtime(_nsec)?\(&self\) -> i64 \{[^}]*\}', '', s)
    return s
edit("os/aros/fs.rs", drop_crtime)

# 8. thread names: the opt-out above removes unix::set_name; provide a no-op.
edit("sys/thread/mod.rs", lambda s: add_after(s, '        pub use unix::set_name;', '\n        #[cfg(target_os = "aros")]\n        pub fn set_name(_name: &crate::ffi::CStr) {}'))

# 9a. linger has 8-byte fields on AROS; the libc struct carries padding fields.
edit("sys/net/connection/socket/unix.rs", lambda s: s.replace(
    '            l_linger: cmp::min(linger.unwrap_or_default().as_secs(), c_int::MAX as u64) as c_int,\n        };',
    '            l_linger: cmp::min(linger.unwrap_or_default().as_secs(), c_int::MAX as u64) as c_int,\n            #[cfg(target_os = "aros")] __pad0: 0,\n            #[cfg(target_os = "aros")] __pad1: 0,\n        };', 1))

# 9. pid_t is 64-bit on AROS while the process_group API takes i32.
edit("os/unix/process.rs", lambda s: s.replace('self.as_inner_mut().pgroup(pgroup);', 'self.as_inner_mut().pgroup(pgroup as _);', 1))
# 9b. errno lives behind ___geterrnoptr() on AROS.
edit("sys/io/error/unix.rs", lambda s: add_after(s, '    #[cfg_attr(target_os = "haiku", link_name = "_errnop")]', '\n    #[cfg_attr(target_os = "aros", link_name = "___geterrnoptr")]'))

# 9c. Skip the standard-fd sanitising: our poll() is bsdsocket's WaitSelect and
#     does not accept console file descriptors, and stdio is always open on AROS.
def skip_fd_sanitise(s):
    s = s.replace('            target_os = "rtems",\n            // The poll on Darwin', '            target_os = "rtems",\n            target_os = "aros",\n            // The poll on Darwin', 1)
    s = s.replace('            target_os = "horizon",\n            target_os = "vita",\n        )))]\n        {', '            target_os = "horizon",\n            target_os = "vita",\n            target_os = "aros",\n        )))]\n        {', 1)
    return s
edit("sys/pal/unix/mod.rs", skip_fd_sanitise)

# 10. std/build.rs keeps a list of supported OSes; anything else gets restricted_std.
bp = os.path.join(STD, "..", "build.rs")
b = open(bp).read()
if '"aros"' not in b:
    b = b.replace('        || target_os == "haiku"', '        || target_os == "haiku"\n        || target_os == "aros"', 1)
    open(bp, "w").write(b); print("  patched build.rs (supported OS list)")

open(marker, "w").write("aros std patch applied\n")
print("done")
