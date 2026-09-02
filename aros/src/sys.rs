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

    // --- exec.library -------------------------------------------------------
    // Reached through csrc/execglue.c rather than by their own names: libexec.a
    // keeps every exec stub in one object that also defines `close`, so naming
    // AllocMem here would drag in a duplicate of the C library's `close`.
    #[link_name = "aros_glue_AllocMem"]
    pub fn AllocMem(size: u64, requirements: u64) -> *mut u8;
    #[link_name = "aros_glue_FreeMem"]
    pub fn FreeMem(mem: *mut u8, size: u64);
}

/// `MEMF_ANY`: let the system choose.
pub const MEMF_ANY: u64 = 0;
/// `MEMF_CLEAR`: zero the block before returning it.
pub const MEMF_CLEAR: u64 = 1 << 16;
