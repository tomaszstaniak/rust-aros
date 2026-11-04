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

# 3. random: route AROS through the libc-aros getentropy adapter.
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
    '            l_linger: cmp::min(linger.unwrap_or_default().as_secs(), c_int::MAX as u64) as c_int,\n            #[cfg(all(target_os = "aros", not(target_abi = "abiv1")))] __pad0: 0,\n            #[cfg(all(target_os = "aros", not(target_abi = "abiv1")))] __pad1: 0,\n        };', 1))

# 9. pid_t is 64-bit on AROS while the process_group API takes i32.
edit("os/unix/process.rs", lambda s: s.replace('self.as_inner_mut().pgroup(pgroup);', 'self.as_inner_mut().pgroup(pgroup as _);', 1))
# 9b. errno lives behind ___geterrnoptr() on ABIv11 and behind stdc.library's
#     __stdc_geterrnoptr() on mainline (x86_64 ABIv1, target_abi "abiv1").
edit("sys/io/error/unix.rs", lambda s: add_after(s, '    #[cfg_attr(target_os = "haiku", link_name = "_errnop")]', '\n    #[cfg_attr(all(target_os = "aros", not(target_abi = "abiv1")), link_name = "___geterrnoptr")]\n    #[cfg_attr(all(target_os = "aros", target_abi = "abiv1"), link_name = "__stdc_geterrnoptr")]'))

# 9c. Skip the standard-fd sanitising: our poll() is bsdsocket's WaitSelect and
#     does not accept console file descriptors, and stdio is always open on AROS.
def skip_fd_sanitise(s):
    s = s.replace('            target_os = "rtems",\n            // The poll on Darwin', '            target_os = "rtems",\n            target_os = "aros",\n            // The poll on Darwin', 1)
    s = s.replace('            target_os = "horizon",\n            target_os = "vita",\n        )))]\n        {', '            target_os = "horizon",\n            target_os = "vita",\n            target_os = "aros",\n        )))]\n        {', 1)
    return s
edit("sys/pal/unix/mod.rs", skip_fd_sanitise)

# AROS implements FD_CLOEXEC through fcntl, not the BSD FIOCLEX ioctl.
# Both complementary cfg lists must choose the same implementation.
def cloexec_with_fcntl(s):
    first = s.index('    pub fn set_cloexec(')
    start = s.rfind('    #[cfg(', 0, first)
    end = s.index('    pub fn set_nonblocking(', first)
    assert start >= 0, "set_cloexec cfg missing"
    block = s[start:end]
    anchor = '        target_os = "haiku",\n'
    assert block.count(anchor) == 2, "set_cloexec cfg anchors changed"
    block = block.replace(anchor, anchor + '        target_os = "aros",\n')
    return s[:start] + block + s[end:]
edit("sys/fd/unix.rs", cloexec_with_fcntl)

# 9d. The null device is NIL: on AROS.
edit("sys/process/unix/common.rs", lambda s: add_after(s, '    target_os = "vxworks" => {\n        const DEV_NULL: &CStr = c"/null";\n    }', '\n    target_os = "aros" => {\n        const DEV_NULL: &CStr = c"NIL:";\n    }'))

# 9e. read_output needs poll() on non-blocking pipes, which AROS cannot do;
#     substitute a thread-based version (source kept beside this script).
HERE = os.path.dirname(os.path.abspath(__file__))
_ro = open(os.path.join(HERE, "read_output_aros.rs")).read()
edit("sys/process/unix/common.rs", lambda s: s.replace("pub fn read_output(\n    out: ChildPipe,", _ro + "pub fn read_output(\n    out: ChildPipe,", 1))

# 9f. pthread_condattr_setclock does not exist on AROS. std asserts on its
#     result and then computes condvar deadlines against the clock it thinks
#     it selected, so claiming success makes every timed wait use a monotonic
#     deadline against a realtime clock -- timers then never fire. Put AROS in
#     the list of platforms that keep CLOCK_REALTIME instead.
def condvar_realtime(s):
    # the default impl excludes these platforms, the fallback impl includes them
    s = s.replace('    target_os = "redox",\n    target_os = "teeos",\n)))]',
                  '    target_os = "redox",\n    target_os = "teeos",\n    target_os = "aros",\n)))]', 1)
    s = s.replace('    target_os = "redox",\n    target_os = "teeos",\n))]',
                  '    target_os = "redox",\n    target_os = "teeos",\n    target_os = "aros",\n))]', 1)
    return s
edit("sys/pal/unix/sync/condvar.rs", condvar_realtime)

# 9g. available_parallelism must not fail on AROS: crates unwrap it (vello_cpu
#     0.2.0 render.rs). Mainline posixc answers _SC_NPROCESSORS_ONLN (since
#     39486f585d); any failure or the ABIv11 SDK, which has no such selector,
#     reports 1: exec schedules these targets on one CPU. Kernel APIC counts
#     are not CPUs exec uses and are not consulted. (The ARM PAL in
#     sys/thread/aros.rs already returns 1.)
def parallelism_aros(s):
    anchor = '        target_os = "haiku" => {'
    assert s.count(anchor) == 1, "available_parallelism haiku arm changed"
    return s.replace(anchor, """        target_os = "aros" => {
            #[cfg(target_abi = "abiv1")]
            let cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
            #[cfg(not(target_abi = "abiv1"))]
            let cpus: libc::c_long = 1;
            Ok(NonZero::new(if cpus > 0 { cpus as usize } else { 1 }).unwrap())
        }
""" + anchor, 1)
edit("sys/thread/unix.rs", parallelism_aros)

# 9h. File::lock/lock_shared/try_lock/try_lock_shared/unlock use flock() on a
#     fixed OS list. posixc has flock() on every AROS SDK; libc-aros routes it
#     through compat.c, which gives POSIX per-descriptor semantics in-process.
def flock_aros(s):
    anchor = '                target_vendor = "apple",\n            ) => {\n                cvt(unsafe { libc::flock('
    assert s.count(anchor) == 3, "flock lock/lock_shared/unlock arms changed"
    s = s.replace(anchor, anchor.replace('target_vendor = "apple",\n', 'target_vendor = "apple",\n                target_os = "aros",\n', 1))
    anchor = '                target_vendor = "apple",\n            ) => {\n                let result =\n                    cvt(unsafe { libc::flock('
    assert s.count(anchor) == 2, "flock try_lock arms changed"
    return s.replace(anchor, anchor.replace('target_vendor = "apple",\n', 'target_vendor = "apple",\n                target_os = "aros",\n', 1))
edit("sys/fs/unix.rs", flock_aros)

# 9g. home_dir: HOME only, never the user database. posixc's getpwuid_r opens
#     usergroup.library on the calling task's posixc base. From a pthread
#     worker that keeps the worker's base open until the program's main task
#     closes usergroup at exit, after dos.library has freed the worker's NIL:
#     handles; stdcio's __close_stdio then writes into them (mainline v1,
#     nightly 20261031). AROS joins the targets whose fallback returns None.
def home_without_userdb(s):
    anchor = '        target_os = "nuttx",\n'
    assert s.count(anchor) == 2, "home_dir fallback cfg lists changed"
    return s.replace(anchor, anchor + '        target_os = "aros",\n')
edit("sys/paths/unix.rs", home_without_userdb)

# 10. std/build.rs keeps a list of supported OSes; anything else gets restricted_std.
bp = os.path.join(STD, "..", "build.rs")
b = open(bp).read()
if '"aros"' not in b:
    b = b.replace('        || target_os == "haiku"', '        || target_os == "haiku"\n        || target_os == "aros"', 1)
    open(bp, "w").write(b); print("  patched build.rs (supported OS list)")

open(marker, "w").write("aros std patch applied\n")
print("done")

# 11. paths: an AmigaDOS volume or assign ends with ':' (`RAM:`, `Work:`).
#     Pushing onto one must not add '/', which there means "parent", and a
#     pushed path that names its own volume (`RAM:x`) replaces the base, as an
#     absolute path does elsewhere. Path::is_absolute is left as it is.
def aros_volume_push(s):
    anchor = """            need_sep = false
        }

        let need_clear = if cfg!(target_os = "cygwin") {"""
    assert anchor in s, "PathBuf::_push changed shape"
    s = s.replace(anchor, """            need_sep = false
        }

        // AROS: `RAM:` + `x` is `RAM:x`; `RAM:/x` would be RAM:'s parent.
        if cfg!(target_os = "aros") && buf.last() == Some(&b':') {
            need_sep = false
        }
        // AROS: a path naming its volume (`Work:x`) does not depend on `self`.
        let aros_volume = cfg!(target_os = "aros")
            && path.inner.as_encoded_bytes().split(|&b| b == b'/').next().is_some_and(|first| first.contains(&b':'));

        let need_clear = aros_volume || if cfg!(target_os = "cygwin") {""", 1)
    return s
edit("path.rs", aros_volume_push)
