//! Raw declarations. Everything AROS exposes to C is a plain symbol, so these
//! are ordinary `extern "C"` items — no register-argument trickery needed.
use core::ffi::c_void;

extern "C" {
    // --- C library (crt.library / stdlib.library via the SDK) ---------------
    pub fn posix_memalign(memptr: *mut *mut c_void, alignment: usize, size: usize) -> i32;
    pub fn free(ptr: *mut c_void);
    pub fn exit(code: i32) -> !;

    // --- dos.library --------------------------------------------------------
    pub fn PutStr(s: *const u8) -> i32;
    pub fn Delay(ticks: u32);

    // Files. `BPTR` is a BCPL pointer, which on AROS is a plain machine word;
    // it is opaque here and only ever handed back to dos.library. Zero is the
    // failure value for `Open`, and it is *not* a valid handle — so a handle
    // is `NonZero`-shaped even though the C type is not.
    pub fn Open(name: *const u8, access_mode: i32) -> i64;
    pub fn Close(file: i64) -> i32;
    /// Returns the count read, 0 at end of file, or -1 on error. **A short
    /// read is not an error and not the end**: it must be looped on.
    pub fn Read(file: i64, buffer: *mut u8, length: i64) -> i64;
    /// Returns the count written, or -1 on error. Short writes happen the
    /// same way and must be looped on too.
    pub fn Write(file: i64, buffer: *const u8, length: i64) -> i64;
    pub fn Rename(old_name: *const u8, new_name: *const u8) -> i32;
    pub fn DeleteFile(name: *const u8) -> i32;
    pub fn CreateDir(name: *const u8) -> i64;
    pub fn Lock(name: *const u8, access_mode: i32) -> i64;
    pub fn UnLock(lock: i64);
    pub fn IoErr() -> i64;

    // --- exec.library -------------------------------------------------------
    // Reached through csrc/execglue.c rather than by their own names: libexec.a
    // keeps every exec stub in one object that also defines `close`, so naming
    // AllocMem here would drag in a duplicate of the C library's `close`.
    #[link_name = "aros_glue_AllocMem"]
    pub fn AllocMem(size: u64, requirements: u64) -> *mut u8;
    #[link_name = "aros_glue_FreeMem"]
    pub fn FreeMem(mem: *mut u8, size: u64);
}

/// `SHARED_LOCK`: look, do not change.
pub const SHARED_LOCK: i32 = -2;

/// `MODE_OLDFILE`: open an existing file for reading and writing.
pub const MODE_OLDFILE: i32 = 1005;
/// `MODE_NEWFILE`: create it, or truncate it if it is there.
pub const MODE_NEWFILE: i32 = 1006;

/// `MEMF_ANY`: let the system choose.
pub const MEMF_ANY: u64 = 0;
/// `MEMF_CLEAR`: zero the block before returning it.
pub const MEMF_CLEAR: u64 = 1 << 16;
