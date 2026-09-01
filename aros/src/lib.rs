//! Write AROS x86_64 programs in Rust.
//!
//! AROS has no Rust `std`, so this crate supplies the pieces a normal program
//! expects: a global allocator (so `Vec`, `String` and friends work), `print!`
//! and `println!`, a panic handler, and thin bindings to the parts of the AROS
//! API you reach for first.
//!
//! A complete program:
//!
//! ```ignore
//! #![no_std]
//! #![no_main]
//! extern crate alloc;
//! use aros::{aros_main, println};
//!
//! fn main() {
//!     println!("hello from Rust on AROS");
//! }
//! aros_main!(main);
//! ```
//!
//! The SDK's C startup stays in charge of process setup; `aros_main!` just
//! exposes your function to it under the name it expects.
#![no_std]

extern crate alloc;

pub mod exec;
pub mod print;
pub mod sys;

#[cfg(feature = "sdl")]
pub mod sdl;

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

/// Rust's allocator, handed over to the SDK's C library.
struct ArosAlloc;

unsafe impl GlobalAlloc for ArosAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut p: *mut c_void = core::ptr::null_mut();
        // posix_memalign wants an alignment that is a multiple of sizeof(void*)
        let align = layout.align().max(core::mem::size_of::<usize>());
        if sys::posix_memalign(&mut p, align, layout.size()) != 0 {
            core::ptr::null_mut()
        } else {
            p as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        sys::free(ptr as *mut c_void);
    }
}

#[global_allocator]
static GLOBAL: ArosAlloc = ArosAlloc;

/// Panics print and end the program: there is no unwinder on AROS, and a task
/// that keeps running after a panic can take the whole machine with it.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("panic: {}", info.message());
    if let Some(loc) = info.location() {
        println!("  at {}:{}", loc.file(), loc.line());
    }
    unsafe { sys::exit(20) }   // 20 = FAIL in AmigaDOS terms
}

/// Declare your entry point.
///
/// AROS programs are started by the SDK's C runtime, which calls `main`. This
/// macro exposes the function you name under that symbol.
#[macro_export]
macro_rules! aros_main {
    ($f:ident) => {
        #[no_mangle]
        pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
            $f();
            0
        }
    };
}

/// Sleep for roughly `ms` milliseconds (dos.library ticks are 1/50 s).
pub fn delay_ms(ms: u32) {
    unsafe { sys::Delay((ms * 50 / 1000).max(1)) }
}
