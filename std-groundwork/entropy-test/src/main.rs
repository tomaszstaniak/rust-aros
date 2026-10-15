//! Bounded x86_64/v11 runtime checks for the real jitterentropy adapter.
#![no_std]
#![no_main]
extern crate alloc;
use aros::{aros_main, println};
use core::ffi::c_void;

extern "C" fn worker(_: *mut c_void) -> *mut c_void {
    let mut bytes = [0u8; 32];
    if unsafe { libc::getentropy(bytes.as_mut_ptr().cast(), bytes.len()) } == 0 {
        core::ptr::null_mut()
    } else {
        1usize as *mut c_void
    }
}
fn run() {
    unsafe {
        assert_eq!(libc::getentropy(core::ptr::null_mut(), 0), 0);
        let mut bytes = [0u8; 257];
        assert_eq!(libc::getentropy(bytes.as_mut_ptr().cast(), 257), -1);
        assert_eq!(*libc::__errno(), libc::EIO);
        // The first actual source reads race through lazy initialization.
        let mut threads = [0; 4];
        for tid in &mut threads {
            assert_eq!(libc::pthread_create(tid, core::ptr::null(), worker, core::ptr::null_mut()), 0);
        }
        for tid in threads {
            let mut result = core::ptr::null_mut();
            assert_eq!(libc::pthread_join(tid, &mut result), 0);
            assert!(result.is_null(), "worker entropy read failed");
        }
        let mut previous = [0u8; 256];
        let rc = libc::getentropy(previous.as_mut_ptr().cast(), previous.len());
        println!("source-read rc={} errno={}", rc, *libc::__errno());
        assert_eq!(rc, 0, "runtime entropy source rejected this environment");
        assert_eq!(libc::getentropy(bytes.as_mut_ptr().cast(), 256), 0);
        assert_ne!(&previous[..], &bytes[..256], "repeated output (smoke check only)");
        println!("ENTROPY_OK: bounds, repeated reads, four joined pthread readers");
    }
}
aros_main!(run);
