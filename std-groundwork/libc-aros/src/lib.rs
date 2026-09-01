//! Minimal `libc` bindings for AROS x86_64.
//!
//! Every type layout here was read out of DWARF produced by the AROS cross
//! compiler, and every constant was expanded from the SDK headers, rather
//! than copied from another platform. That matters more than it sounds:
//! AROS has `O_RDONLY == 1` (not 0), BSD-style errno values, a 4-byte
//! `time_t`, an 8-byte `pid_t` and a 4-byte `pthread_t`. A Linux-shaped
//! guess would compile and then quietly misbehave.
//!
//! Regenerate with `probe/` in this repository.
#![no_std]
#![allow(non_camel_case_types, non_snake_case, missing_docs, unused_imports)]
// When built as part of std itself, core comes from the std workspace.
#![cfg_attr(feature = "rustc-dep-of-std", feature(no_core))]
#![cfg_attr(feature = "rustc-dep-of-std", no_core)]

#[cfg(feature = "rustc-dep-of-std")]
extern crate rustc_std_workspace_core as core;
#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_imports)]
use core::prelude::v1::*;

pub use core::ffi::{c_char, c_int, c_long, c_short, c_uchar, c_uint, c_ulong, c_ushort, c_void};

// --- scalar types (sizes verified against the SDK) ---------------------
pub type off_t = i64;
pub type blksize_t = i64;
pub type blkcnt_t = i64;
pub type clockid_t = i32;
pub type dev_t = u64;
pub type gid_t = u32;
pub type ino_t = u64;
pub type mode_t = u16;
pub type nlink_t = u16;
pub type pid_t = i64;
pub type size_t = usize;
pub type ssize_t = isize;
pub type time_t = i32;
pub type uid_t = u32;
pub type pthread_t = u32;
pub type pthread_key_t = u32;
pub type suseconds_t = i32;

// --- structures -------------------------------------------------------
/// `tv_sec` is only 32 bits on AROS, so times run out in 2038.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct timespec {
    pub tv_sec: time_t,
    pub tv_nsec: c_long,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct timeval {
    pub tv_sec: time_t,
    pub tv_usec: suseconds_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

/// 128 bytes; `st_flags` and `st_gen` are left opaque because nothing reads them.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct stat {
    pub st_dev: dev_t,
    pub st_ino: ino_t,
    pub st_mode: mode_t,
    pub st_nlink: nlink_t,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    __pad0: u32,
    pub st_rdev: dev_t,
    pub st_size: off_t,
    // Each of these is a `struct timespec` in the C header; they are split
    // into the two fields std expects (`st_atime` + `st_atime_nsec`), with
    // identical layout: a 4-byte time_t, padding, an 8-byte nanosecond count.
    pub st_atime: time_t,
    __pad_a: u32,
    pub st_atime_nsec: c_long,
    pub st_mtime: time_t,
    __pad_m: u32,
    pub st_mtime_nsec: c_long,
    pub st_ctime: time_t,
    __pad_c: u32,
    pub st_ctime_nsec: c_long,
    pub st_blksize: blksize_t,
    pub st_blocks: blkcnt_t,
    __unused: [u8; 16],
}

/// 280 bytes, with the name starting at offset 19.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct dirent {
    pub d_ino: ino_t,
    pub d_off: off_t,
    pub d_reclen: c_ushort,
    pub d_type: c_uchar,
    pub d_name: [c_char; 261],
}

#[repr(C)]
pub struct DIR {
    _opaque: [u8; 0],
}

#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct pthread_attr_t {
    __opaque: [u8; 32],
}
impl pthread_attr_t {
    pub const fn zeroed() -> Self { Self { __opaque: [0; 32] } }
}

#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct pthread_mutexattr_t {
    __opaque: [u8; 8],
}
impl pthread_mutexattr_t {
    pub const fn zeroed() -> Self { Self { __opaque: [0; 8] } }
}

#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct pthread_mutex_t {
    __opaque: [u8; 120],
}
impl pthread_mutex_t {
    pub const fn zeroed() -> Self { Self { __opaque: [0; 120] } }
}

#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct pthread_condattr_t {
    __opaque: [u8; 4],
}
impl pthread_condattr_t {
    pub const fn zeroed() -> Self { Self { __opaque: [0; 4] } }
}

#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct pthread_cond_t {
    __opaque: [u8; 136],
}
impl pthread_cond_t {
    pub const fn zeroed() -> Self { Self { __opaque: [0; 136] } }
}

#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct pthread_rwlock_t {
    __opaque: [u8; 104],
}
impl pthread_rwlock_t {
    pub const fn zeroed() -> Self { Self { __opaque: [0; 104] } }
}

// --- constants (expanded from the SDK headers) ------------------------
pub const O_RDONLY: c_int = 1;
pub const O_WRONLY: c_int = 2;
pub const O_RDWR: c_int = 3;
pub const O_APPEND: c_int = 1024;
pub const O_CREAT: c_int = 64;
pub const O_EXCL: c_int = 128;
pub const O_TRUNC: c_int = 512;
pub const O_NONBLOCK: c_int = 2048;
pub const O_ACCMODE: c_int = 3;
pub const F_GETFD: c_int = 2;
pub const F_SETFD: c_int = 3;
pub const F_GETFL: c_int = 4;
pub const F_SETFL: c_int = 5;
pub const F_DUPFD: c_int = 0;
pub const F_DUPFD_CLOEXEC: c_int = 1;
pub const FD_CLOEXEC: c_int = 1;
pub const S_IFMT: mode_t = 61440;
pub const S_IFREG: mode_t = 32768;
pub const S_IFDIR: mode_t = 16384;
pub const S_IFLNK: mode_t = 40960;
pub const S_IFIFO: mode_t = 4096;
pub const S_IFCHR: mode_t = 8192;
pub const S_IFBLK: mode_t = 24576;
pub const S_IFSOCK: mode_t = 49152;
pub const S_IRWXU: mode_t = 448;
pub const S_IRUSR: mode_t = 256;
pub const S_IWUSR: mode_t = 128;
pub const S_IXUSR: mode_t = 64;
pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;
pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;
pub const CLOCK_REALTIME: c_int = 2;
pub const CLOCK_MONOTONIC: c_int = 0;
pub const DT_UNKNOWN: c_uchar = 0;
pub const DT_FIFO: c_uchar = 1;
pub const DT_CHR: c_uchar = 2;
pub const DT_DIR: c_uchar = 4;
pub const DT_BLK: c_uchar = 6;
pub const DT_REG: c_uchar = 8;
pub const DT_LNK: c_uchar = 10;
pub const DT_SOCK: c_uchar = 12;
pub const PTHREAD_CREATE_JOINABLE: c_int = 0;
pub const PTHREAD_CREATE_DETACHED: c_int = 1;
pub const PTHREAD_MUTEX_NORMAL: c_int = 0;
pub const PTHREAD_MUTEX_RECURSIVE: c_int = 1;
pub const PTHREAD_MUTEX_DEFAULT: c_int = 0;
pub const PTHREAD_STACK_MIN: size_t = 40960;
pub const SIGSEGV: c_int = 11;
pub const SIGBUS: c_int = 10;
pub const SIGPIPE: c_int = 13;
pub const SIGABRT: c_int = 6;
pub const SIGINT: c_int = 2;
pub const EPERM: c_int = 1;
pub const ENOENT: c_int = 2;
pub const ESRCH: c_int = 3;
pub const EINTR: c_int = 4;
pub const EIO: c_int = 5;
pub const ENXIO: c_int = 6;
pub const E2BIG: c_int = 7;
pub const EBADF: c_int = 9;
pub const ECHILD: c_int = 10;
pub const EAGAIN: c_int = 35;
pub const ENOMEM: c_int = 12;
pub const EACCES: c_int = 13;
pub const EFAULT: c_int = 14;
pub const EBUSY: c_int = 16;
pub const EEXIST: c_int = 17;
pub const EXDEV: c_int = 18;
pub const ENODEV: c_int = 19;
pub const ENOTDIR: c_int = 20;
pub const EISDIR: c_int = 21;
pub const EINVAL: c_int = 22;
pub const ENFILE: c_int = 23;
pub const EMFILE: c_int = 24;
pub const ENOTTY: c_int = 25;
pub const EFBIG: c_int = 27;
pub const ENOSPC: c_int = 28;
pub const ESPIPE: c_int = 29;
pub const EROFS: c_int = 30;
pub const EMLINK: c_int = 31;
pub const EPIPE: c_int = 32;
pub const EDOM: c_int = 33;
pub const ERANGE: c_int = 34;
pub const EDEADLK: c_int = 11;
pub const ENAMETOOLONG: c_int = 63;
pub const ENOSYS: c_int = 78;
pub const ENOTEMPTY: c_int = 66;
pub const ELOOP: c_int = 62;
pub const EWOULDBLOCK: c_int = 35;
pub const EOVERFLOW: c_int = 84;
pub const ETIMEDOUT: c_int = 60;
pub const ECONNREFUSED: c_int = 61;
pub const ECONNRESET: c_int = 54;
pub const EADDRINUSE: c_int = 48;
pub const ENOTCONN: c_int = 57;
pub const EALREADY: c_int = 37;
pub const EINPROGRESS: c_int = 36;
pub const EADDRNOTAVAIL: c_int = 49;
pub const ENOTSOCK: c_int = 38;
pub const EAFNOSUPPORT: c_int = 47;
pub const EHOSTUNREACH: c_int = 65;
pub const ENETUNREACH: c_int = 51;
pub const ECANCELED: c_int = 87;
pub const EMSGSIZE: c_int = 40;

// Not defined by AROS, listed so the gap is explicit:
//   O_CLOEXEC, O_DIRECTORY, O_NOFOLLOW, AT_FDCWD, _SC_PAGESIZE, _SC_NPROCESSORS_ONLN

// --- functions (each verified to exist in the SDK link libraries) ------
extern "C" {
    /// AROS' C library exports this only under an internal name: the plain
    /// `open` symbol belongs to exec.library and is a different function.
    #[link_name = "__open_CrtBase_wrapper"]
    pub fn open(path: *const c_char, oflag: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;
    pub fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    pub fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn isatty(fd: c_int) -> c_int;
    pub fn ftruncate(fd: c_int, length: off_t) -> c_int;
    pub fn fsync(fd: c_int) -> c_int;
    pub fn stat(path: *const c_char, buf: *mut stat) -> c_int;
    pub fn fstat(fd: c_int, buf: *mut stat) -> c_int;
    pub fn lstat(path: *const c_char, buf: *mut stat) -> c_int;
    pub fn mkdir(path: *const c_char, mode: mode_t) -> c_int;
    pub fn rmdir(path: *const c_char) -> c_int;
    pub fn unlink(path: *const c_char) -> c_int;
    pub fn rename(old: *const c_char, new: *const c_char) -> c_int;
    pub fn link(old: *const c_char, new: *const c_char) -> c_int;
    pub fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int;
    pub fn readlink(path: *const c_char, buf: *mut c_char, bufsiz: size_t) -> ssize_t;
    pub fn realpath(path: *const c_char, resolved: *mut c_char) -> *mut c_char;
    pub fn chdir(path: *const c_char) -> c_int;
    pub fn getcwd(buf: *mut c_char, size: size_t) -> *mut c_char;
    pub fn opendir(name: *const c_char) -> *mut DIR;
    pub fn readdir(dirp: *mut DIR) -> *mut dirent;
    pub fn closedir(dirp: *mut DIR) -> c_int;
    pub fn getenv(name: *const c_char) -> *mut c_char;
    pub fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    pub fn unsetenv(name: *const c_char) -> c_int;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strlen(s: *const c_char) -> size_t;
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn malloc(size: size_t) -> *mut c_void;
    pub fn calloc(n: size_t, size: size_t) -> *mut c_void;
    pub fn realloc(p: *mut c_void, size: size_t) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn posix_memalign(memptr: *mut *mut c_void, align: size_t, size: size_t) -> c_int;
    pub fn exit(code: c_int) -> !;
    pub fn _exit(code: c_int) -> !;
    pub fn abort() -> !;
    pub fn clock_gettime(clk: clockid_t, tp: *mut timespec) -> c_int;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;
    pub fn nanosleep(req: *const timespec, rem: *mut timespec) -> c_int;
    pub fn time(t: *mut time_t) -> time_t;
    pub fn localtime_r(t: *const time_t, tm: *mut tm) -> *mut tm;
    pub fn gmtime_r(t: *const time_t, tm: *mut tm) -> *mut tm;
    pub fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t;
    pub fn execv(path: *const c_char, argv: *const *const c_char) -> c_int;
    pub fn execvp(file: *const c_char, argv: *const *const c_char) -> c_int;
    pub fn kill(pid: pid_t, sig: c_int) -> c_int;
}

// pthreads live in their own link library on AROS.
#[link(name = "pthread")]
extern "C" {
    pub fn sched_yield() -> c_int;
    pub fn pthread_create(t: *mut pthread_t, attr: *const pthread_attr_t, f: extern "C" fn(*mut c_void) -> *mut c_void, arg: *mut c_void) -> c_int;
    pub fn pthread_join(t: pthread_t, res: *mut *mut c_void) -> c_int;
    pub fn pthread_detach(t: pthread_t) -> c_int;
    pub fn pthread_self() -> pthread_t;
    pub fn pthread_attr_init(a: *mut pthread_attr_t) -> c_int;
    pub fn pthread_attr_destroy(a: *mut pthread_attr_t) -> c_int;
    pub fn pthread_attr_setstacksize(a: *mut pthread_attr_t, s: size_t) -> c_int;
    pub fn pthread_attr_setdetachstate(a: *mut pthread_attr_t, s: c_int) -> c_int;
    pub fn pthread_mutex_init(m: *mut pthread_mutex_t, a: *const pthread_mutexattr_t) -> c_int;
    pub fn pthread_mutex_destroy(m: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_lock(m: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_trylock(m: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_unlock(m: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_cond_init(c: *mut pthread_cond_t, a: *const pthread_condattr_t) -> c_int;
    pub fn pthread_cond_destroy(c: *mut pthread_cond_t) -> c_int;
    pub fn pthread_cond_wait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, t: *const timespec) -> c_int;
    pub fn pthread_cond_signal(c: *mut pthread_cond_t) -> c_int;
    pub fn pthread_cond_broadcast(c: *mut pthread_cond_t) -> c_int;
    pub fn pthread_key_create(k: *mut pthread_key_t, d: Option<extern "C" fn(*mut c_void)>) -> c_int;
    pub fn pthread_key_delete(k: pthread_key_t) -> c_int;
    pub fn pthread_getspecific(k: pthread_key_t) -> *mut c_void;
    pub fn pthread_setspecific(k: pthread_key_t, v: *const c_void) -> c_int;
}

// --- more scalar types --------------------------------------------------
pub type socklen_t = u32;
pub type sa_family_t = u8;
pub type in_addr_t = u32;
pub type in_port_t = u16;
pub type sighandler_t = usize;
pub type nfds_t = c_ulong;
pub use core::ffi::c_double;
pub type uintptr_t = usize;
pub type intptr_t = isize;
pub type c_longlong = i64;
pub type c_ulonglong = u64;
pub type c_float = f32;

// --- socket structures (BSD layout with a leading length byte) ----------
#[repr(C)] #[derive(Copy, Clone)] pub struct sockaddr { pub sa_len: u8, pub sa_family: sa_family_t, pub sa_data: [c_char; 14] }
#[repr(C)] #[derive(Copy, Clone)] pub struct in_addr { pub s_addr: in_addr_t }
#[repr(C)] #[derive(Copy, Clone)] pub struct in6_addr { pub s6_addr: [u8; 16] }
#[repr(C)] #[derive(Copy, Clone)] pub struct sockaddr_in { pub sin_len: u8, pub sin_family: sa_family_t, pub sin_port: in_port_t, pub sin_addr: in_addr, pub sin_zero: [u8; 8] }
#[repr(C)] #[derive(Copy, Clone)] pub struct sockaddr_in6 { pub sin6_len: u8, pub sin6_family: sa_family_t, pub sin6_port: in_port_t, pub sin6_flowinfo: u32, pub sin6_addr: in6_addr, pub sin6_scope_id: u32 }
#[repr(C)] #[derive(Copy, Clone)] pub struct sockaddr_un { pub sun_len: u8, pub sun_family: sa_family_t, pub sun_path: [c_char; 104] }
#[repr(C, align(8))] #[derive(Copy, Clone)] pub struct sockaddr_storage { pub ss_len: u8, pub ss_family: sa_family_t, __pad: [u8; 126] }
/// Both fields are 8-byte `long` in the C header; std expects `c_int`, so
/// each is declared as the int followed by explicit padding (little-endian:
/// the low half is the value).
#[repr(C)] #[derive(Copy, Clone)] pub struct linger { pub l_onoff: c_int, pub __pad0: c_int, pub l_linger: c_int, pub __pad1: c_int }
/// Same treatment: `long` fields in C, `c_int` + padding here.
#[repr(C)] pub struct addrinfo { pub ai_flags: c_int, __p0: c_int, pub ai_family: c_int, __p1: c_int, pub ai_socktype: c_int, __p2: c_int, pub ai_protocol: c_int, __p3: c_int, pub ai_addrlen: socklen_t, __p4: u32, pub ai_canonname: *mut c_char, pub ai_addr: *mut sockaddr, pub ai_next: *mut addrinfo }
#[repr(C)] #[derive(Copy, Clone)] pub struct pollfd { pub fd: c_int, pub events: c_short, pub revents: c_short }
#[repr(C)] #[derive(Copy, Clone)] pub struct iovec { pub iov_base: *mut c_void, pub iov_len: size_t }
#[repr(C)] #[derive(Copy, Clone)] pub struct sigset_t { __bits: [u32; 4] }
#[repr(C)] pub struct passwd { pub pw_name: *mut c_char, pub pw_passwd: *mut c_char, pub pw_uid: uid_t, pub pw_gid: gid_t, pub pw_gecos: *mut c_char, pub pw_dir: *mut c_char, pub pw_shell: *mut c_char }
#[repr(C)] pub struct sigaction { pub sa_handler: sighandler_t, pub sa_mask: sigset_t, pub sa_flags: c_int }
pub const AF_INET: c_int = 2;
pub const AF_INET6: c_int = 24;
pub const IPPROTO_IP: c_int = 0;
pub const IPPROTO_IPV6: c_int = 41;
pub const IPPROTO_UDP: c_int = 17;
pub const IP_TTL: c_int = 4;
pub const IP_MULTICAST_TTL: c_int = 10;
pub const IP_MULTICAST_LOOP: c_int = 11;
pub const IP_ADD_MEMBERSHIP: c_int = 12;
pub const IP_DROP_MEMBERSHIP: c_int = 13;
pub const IPV6_MULTICAST_LOOP: c_int = 11;
pub const IPV6_V6ONLY: c_int = 27;
pub const IPV6_ADD_MEMBERSHIP: c_int = 12;
pub const IPV6_DROP_MEMBERSHIP: c_int = 13;
pub const SO_REUSEADDR: c_int = 4;
pub const SO_BROADCAST: c_int = 0x20;
pub const SO_RCVBUF: c_int = 0x1002;
pub const MSG_NOSIGNAL: c_int = 0;
pub const INADDR_ANY: in_addr_t = 0;
pub const EAI_NONAME: c_int = 8;
#[repr(C)] #[derive(Copy, Clone)] pub struct ip_mreq { pub imr_multiaddr: in_addr, pub imr_interface: in_addr }
#[repr(C)] #[derive(Copy, Clone)] pub struct ipv6_mreq { pub ipv6mr_multiaddr: in6_addr, pub ipv6mr_interface: c_uint }

pub const AF_UNIX: c_int = 1;
pub const EAI_SYSTEM: c_int = 11;
pub const ECONNABORTED: c_int = 53;
pub const EDQUOT: c_int = 69;
pub const EISCONN: c_int = 56;
pub const ENETDOWN: c_int = 50;
pub const ENOTSUP: c_int = 86;
pub const EOPNOTSUPP: c_int = 45;
pub const ESTALE: c_int = 70;
pub const ETXTBSY: c_int = 26;
pub const FIOCLEX: c_ulong = 536897025;
pub const FIONBIO: c_ulong = 2148034174;
pub const IPPROTO_TCP: c_int = 6;
pub const MSG_PEEK: c_int = 2;
pub const S_IRGRP: mode_t = 32;
pub const S_IROTH: mode_t = 4;
pub const S_ISGID: mode_t = 1024;
pub const S_ISUID: mode_t = 2048;
pub const S_ISVTX: mode_t = 512;
pub const S_IWGRP: mode_t = 16;
pub const S_IWOTH: mode_t = 2;
pub const S_IXGRP: mode_t = 8;
pub const S_IXOTH: mode_t = 1;
pub const SIGALRM: c_int = 14;
pub const SIGCHLD: c_int = 20;
pub const SIGCONT: c_int = 19;
pub const SIGFPE: c_int = 8;
pub const SIGHUP: c_int = 1;
pub const SIGILL: c_int = 4;
pub const SIGKILL: c_int = 9;
pub const SIGPROF: c_int = 26;
pub const SIGQUIT: c_int = 3;
pub const SIGSTOP: c_int = 17;
pub const SIGSYS: c_int = 12;
pub const SIGTERM: c_int = 15;
pub const SIGTRAP: c_int = 5;
pub const SIGTSTP: c_int = 18;
pub const SIGTTIN: c_int = 21;
pub const SIGTTOU: c_int = 22;
pub const SIGURG: c_int = 16;
pub const SIGUSR1: c_int = 28;
pub const SIGUSR2: c_int = 29;
pub const SIGVTALRM: c_int = 25;
pub const SIGXCPU: c_int = 23;
pub const SIGXFSZ: c_int = 24;
pub const SO_ERROR: c_int = 4103;
pub const SO_KEEPALIVE: c_int = 8;
pub const SO_LINGER: c_int = 128;
pub const SO_RCVTIMEO: c_int = 4102;
pub const SO_SNDTIMEO: c_int = 4101;
pub const SOCK_DGRAM: c_int = 2;
pub const SOCK_STREAM: c_int = 1;
pub const SOL_SOCKET: c_int = 65535;
pub const SOMAXCONN: c_int = 128;
pub const TCP_NODELAY: c_int = 1;
// --- not provided by AROS; values chosen by this crate ------------------
/// Not a real flag on AROS: file descriptors are not inherited by children
/// created through SystemTagList anyway, so this can be accepted and ignored.
pub const O_CLOEXEC: c_int = 0;
pub const O_DIRECTORY: c_int = 0;
pub const O_NOFOLLOW: c_int = 0;
pub const AT_FDCWD: c_int = -100;
pub const AT_REMOVEDIR: c_int = 0x200;
pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
pub const UTIME_OMIT: c_long = -2;
pub const SHUT_RD: c_int = 0;
pub const SHUT_WR: c_int = 1;
pub const SHUT_RDWR: c_int = 2;
pub const POLLIN: c_short = 0x1;
pub const POLLOUT: c_short = 0x4;
pub const POLLERR: c_short = 0x8;
pub const POLLHUP: c_short = 0x10;
pub const POLLNVAL: c_short = 0x20;
pub const SIGIO: c_int = 23;
pub const SIGWINCH: c_int = 28;
pub const WNOHANG: c_int = 1;
pub const EXIT_SUCCESS: c_int = 0;
pub const EXIT_FAILURE: c_int = 1;
pub const _SC_PAGESIZE: c_int = 8;
pub const _SC_GETPW_R_SIZE_MAX: c_int = 70;
pub const _SC_HOST_NAME_MAX: c_int = 72;
pub const _SC_NPROCESSORS_ONLN: c_int = 84;
pub const PATH_MAX: c_int = 1024;
pub const SIG_DFL: sighandler_t = 0;
pub const SIG_IGN: sighandler_t = 1;
pub const SIG_ERR: sighandler_t = !0;
pub const PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = pthread_mutex_t::zeroed();
pub const PTHREAD_COND_INITIALIZER: pthread_cond_t = pthread_cond_t::zeroed();

// waitpid status decoding, BSD convention (status = exit << 8 | signal)
pub const fn WIFEXITED(s: c_int) -> bool { (s & 0x7f) == 0 }
pub const fn WEXITSTATUS(s: c_int) -> c_int { (s >> 8) & 0xff }
pub const fn WIFSIGNALED(s: c_int) -> bool { ((s & 0x7f) + 1) as i8 >> 1 > 0 }
pub const fn WTERMSIG(s: c_int) -> c_int { s & 0x7f }
pub const fn WIFSTOPPED(s: c_int) -> bool { (s & 0xff) == 0x7f }
pub const fn WSTOPSIG(s: c_int) -> c_int { (s >> 8) & 0xff }
pub const fn WIFCONTINUED(s: c_int) -> bool { s == 0xffff }
pub const fn WCOREDUMP(s: c_int) -> bool { (s & 0x80) != 0 }

// --- functions present in the SDK (second batch) ------------------------
extern "C" {
    pub fn chmod(path: *const c_char, mode: mode_t) -> c_int;
    pub fn fchmod(fd: c_int, mode: mode_t) -> c_int;
    pub fn chown(path: *const c_char, uid: uid_t, gid: gid_t) -> c_int;
    pub fn fchown(fd: c_int, uid: uid_t, gid: gid_t) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dirfd(dirp: *mut DIR) -> c_int;
    pub fn getpid() -> pid_t;
    pub fn getppid() -> pid_t;
    pub fn getuid() -> uid_t;
    pub fn geteuid() -> uid_t;
    pub fn getgid() -> gid_t;
    pub fn getegid() -> gid_t;
    pub fn setuid(uid: uid_t) -> c_int;
    pub fn setgid(gid: gid_t) -> c_int;
    pub fn setsid() -> pid_t;
    pub fn ioctl(fd: c_int, req: c_ulong, ...) -> c_int;
    pub fn sysconf(name: c_int) -> c_long;
    pub fn strnlen(s: *const c_char, n: size_t) -> size_t;
    pub fn signal(sig: c_int, handler: sighandler_t) -> sighandler_t;
    pub fn sigemptyset(set: *mut sigset_t) -> c_int;
    pub fn sigaddset(set: *mut sigset_t, sig: c_int) -> c_int;
    pub fn sigaction(sig: c_int, act: *const sigaction, old: *mut sigaction) -> c_int;
    pub fn writev(fd: c_int, iov: *const iovec, n: c_int) -> ssize_t;
    pub fn fdatasync(fd: c_int) -> c_int;
    pub fn __errno() -> *mut c_int;
    pub fn strerror_r(errnum: c_int, buf: *mut c_char, len: size_t) -> c_int;
    pub fn getentropy(buf: *mut c_void, len: size_t) -> c_int;
    pub fn ftruncate64(fd: c_int, length: off_t) -> c_int;
}
extern "C" {
    pub fn pthread_condattr_setclock(a: *mut pthread_condattr_t, c: clockid_t) -> c_int;
    pub fn pthread_mutexattr_init(a: *mut pthread_mutexattr_t) -> c_int;
    pub fn pthread_mutexattr_destroy(a: *mut pthread_mutexattr_t) -> c_int;
    pub fn pthread_mutexattr_settype(a: *mut pthread_mutexattr_t, t: c_int) -> c_int;
    pub fn pthread_condattr_init(a: *mut pthread_condattr_t) -> c_int;
    pub fn pthread_condattr_destroy(a: *mut pthread_condattr_t) -> c_int;
    pub fn pthread_attr_getstacksize(a: *const pthread_attr_t, s: *mut size_t) -> c_int;
    pub fn pthread_attr_getguardsize(a: *const pthread_attr_t, s: *mut size_t) -> c_int;
    pub fn pthread_getattr_np(t: pthread_t, a: *mut pthread_attr_t) -> c_int;
    pub fn pthread_rwlock_init(l: *mut pthread_rwlock_t, a: *const c_void) -> c_int;
    pub fn pthread_rwlock_destroy(l: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_rdlock(l: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_tryrdlock(l: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_wrlock(l: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_trywrlock(l: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_unlock(l: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_setname_np(t: pthread_t, name: *const c_char) -> c_int;
}

// --- provided by csrc/compat.c ------------------------------------------
// Sockets go through bsdsocket.library's base register; the C side wraps them.
extern "C" {
    #[link_name = "rs_socket"]      pub fn socket(d: c_int, t: c_int, p: c_int) -> c_int;
    #[link_name = "rs_connect"]     pub fn connect(s: c_int, a: *const sockaddr, l: socklen_t) -> c_int;
    #[link_name = "rs_bind"]        pub fn bind(s: c_int, a: *const sockaddr, l: socklen_t) -> c_int;
    #[link_name = "rs_listen"]      pub fn listen(s: c_int, b: c_int) -> c_int;
    #[link_name = "rs_accept"]      pub fn accept(s: c_int, a: *mut sockaddr, l: *mut socklen_t) -> c_int;
    #[link_name = "rs_send"]        pub fn send(s: c_int, b: *const c_void, n: size_t, f: c_int) -> ssize_t;
    #[link_name = "rs_recv"]        pub fn recv(s: c_int, b: *mut c_void, n: size_t, f: c_int) -> ssize_t;
    #[link_name = "rs_sendto"]      pub fn sendto(s: c_int, b: *const c_void, n: size_t, f: c_int, a: *const sockaddr, l: socklen_t) -> ssize_t;
    #[link_name = "rs_recvfrom"]    pub fn recvfrom(s: c_int, b: *mut c_void, n: size_t, f: c_int, a: *mut sockaddr, l: *mut socklen_t) -> ssize_t;
    #[link_name = "rs_shutdown"]    pub fn shutdown(s: c_int, how: c_int) -> c_int;
    #[link_name = "rs_getsockname"] pub fn getsockname(s: c_int, a: *mut sockaddr, l: *mut socklen_t) -> c_int;
    #[link_name = "rs_getpeername"] pub fn getpeername(s: c_int, a: *mut sockaddr, l: *mut socklen_t) -> c_int;
    #[link_name = "rs_setsockopt"]  pub fn setsockopt(s: c_int, lv: c_int, o: c_int, v: *const c_void, l: socklen_t) -> c_int;
    #[link_name = "rs_getsockopt"]  pub fn getsockopt(s: c_int, lv: c_int, o: c_int, v: *mut c_void, l: *mut socklen_t) -> c_int;
    #[link_name = "rs_closesocket"] pub fn closesocket(s: c_int) -> c_int;
    #[link_name = "rs_getaddrinfo"] pub fn getaddrinfo(node: *const c_char, service: *const c_char, hints: *const addrinfo, res: *mut *mut addrinfo) -> c_int;
    #[link_name = "rs_freeaddrinfo"] pub fn freeaddrinfo(res: *mut addrinfo);
    #[link_name = "rs_gethostname"] pub fn gethostname(name: *mut c_char, len: size_t) -> c_int;
    pub fn gai_strerror(err: c_int) -> *const c_char;
    pub fn socketpair(d: c_int, t: c_int, p: c_int, sv: *mut c_int) -> c_int;
    pub fn openat(fd: c_int, path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn mkdirat(fd: c_int, path: *const c_char, mode: mode_t) -> c_int;
    pub fn unlinkat(fd: c_int, path: *const c_char, flags: c_int) -> c_int;
    pub fn linkat(f1: c_int, a: *const c_char, f2: c_int, b: *const c_char, fl: c_int) -> c_int;
    pub fn renameat(f1: c_int, a: *const c_char, f2: c_int, b: *const c_char) -> c_int;
    pub fn fstatat(fd: c_int, path: *const c_char, st: *mut stat, fl: c_int) -> c_int;
    pub fn utimensat(fd: c_int, path: *const c_char, ts: *const timespec, fl: c_int) -> c_int;
    pub fn futimens(fd: c_int, ts: *const timespec) -> c_int;
    pub fn fdopendir(fd: c_int) -> *mut DIR;
    /// vfork: AROS has no fork. Bound directly (see compat.c for why no
    /// wrapper is possible); std only dup2s/closes/chdirs before exec.
    #[link_name = "__vfork_CrtBase_wrapper"]
    pub fn fork() -> pid_t;
    pub fn chroot(path: *const c_char) -> c_int;
    pub fn mkfifo(path: *const c_char, mode: mode_t) -> c_int;
    pub fn killpg(pg: pid_t, sig: c_int) -> c_int;
    pub fn setpgid(a: pid_t, b: pid_t) -> c_int;
    pub fn setgroups(n: c_int, g: *const gid_t) -> c_int;
    pub fn lchown(path: *const c_char, uid: uid_t, gid: gid_t) -> c_int;
    pub fn getpwuid_r(uid: uid_t, pw: *mut passwd, buf: *mut c_char, n: size_t, res: *mut *mut passwd) -> c_int;
    pub fn poll(fds: *mut pollfd, n: nfds_t, timeout: c_int) -> c_int;
    pub fn pread(fd: c_int, b: *mut c_void, n: size_t, off: off_t) -> ssize_t;
    pub fn pwrite(fd: c_int, b: *const c_void, n: size_t, off: off_t) -> ssize_t;
    pub fn readv(fd: c_int, iov: *const iovec, n: c_int) -> ssize_t;
}
