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
#![allow(non_camel_case_types, non_snake_case, missing_docs)]

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
#[derive(Copy, Clone)]
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
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
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
