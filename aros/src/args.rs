//! The command line, as the Shell handed it over.
//!
//! AROS starts a program through the SDK's C runtime, which calls `main(argc,
//! argv)` exactly as C expects. [`crate::aros_main`] keeps them so that a
//! program can read its own arguments — `Game seed 7`, say — instead of
//! having to be rebuilt to be told anything.

use core::ffi::c_char;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

static COUNT: AtomicUsize = AtomicUsize::new(0);
static VECTOR: AtomicPtr<*const u8> = AtomicPtr::new(core::ptr::null_mut());

/// Called once by [`crate::aros_main`], before anything else runs.
///
/// # Safety
///
/// `argv` must be the vector the C runtime passed, valid for `argc` entries
/// and for the life of the program — which is what a C `main` is given.
pub unsafe fn remember(argc: i32, argv: *const *const u8) {
    COUNT.store(argc.max(0) as usize, Ordering::Relaxed);
    VECTOR.store(argv as *mut *const u8, Ordering::Relaxed);
}

/// How many arguments there are, the program's own name included.
pub fn count() -> usize {
    COUNT.load(Ordering::Relaxed)
}

/// One argument, or `None` past the end.
///
/// Argument zero is the program's name, as in C. Anything that is not valid
/// UTF-8 comes back as `None` rather than as mojibake.
pub fn get(index: usize) -> Option<&'static str> {
    if index >= count() {
        return None;
    }
    let vector = VECTOR.load(Ordering::Relaxed);
    if vector.is_null() {
        return None;
    }
    // SAFETY: `remember` was given a vector valid for `count()` entries and
    // for the life of the program, and nothing writes to it afterwards.
    unsafe {
        let string = *vector.add(index);
        if string.is_null() {
            return None;
        }
        let mut len = 0usize;
        while *string.add(len) != 0 {
            len += 1;
        }
        core::str::from_utf8(core::slice::from_raw_parts(string, len)).ok()
    }
}

/// Every argument in order, the program's name first.
pub fn all() -> impl Iterator<Item = &'static str> {
    (0..count()).filter_map(get)
}

/// The argument after `name`, if it is there — `seed 7` gives `"7"`.
pub fn value_of(name: &str) -> Option<&'static str> {
    let at = (0..count()).find(|&i| get(i) == Some(name))?;
    get(at + 1)
}

/// Is `name` among the arguments?
pub fn has(name: &str) -> bool {
    (1..count()).any(|i| get(i) == Some(name))
}

const _: fn() = || {
    // `c_char` is only referenced to document what the vector points at.
    let _: Option<*const c_char> = None;
};
