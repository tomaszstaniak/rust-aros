//! `print!` / `println!` for AROS, written straight onto dos.library's output.
//!
//! The AROS console is not UTF-8, so keep printed text to ASCII.
//!
//! No allocation is involved: text is formatted into a small stack buffer and
//! flushed through `PutStr` whenever it fills up.
use core::fmt::{self, Write};

pub struct Stdout {
    buf: [u8; 256],
    n: usize,
}

impl Stdout {
    pub const fn new() -> Self {
        Stdout { buf: [0; 256], n: 0 }
    }

    pub fn flush(&mut self) {
        if self.n > 0 {
            self.buf[self.n] = 0;
            unsafe { crate::sys::PutStr(self.buf.as_ptr()) };
            self.n = 0;
        }
    }
}

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            if self.n + 1 >= self.buf.len() {
                self.flush();
            }
            self.buf[self.n] = b;
            self.n += 1;
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut out = Stdout::new();
    let _ = fmt::write(&mut out, args);
    out.flush();
}

/// Print to the Shell the program was started from.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::print::_print(format_args!($($arg)*)) };
}

/// Print a line to the Shell the program was started from.
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { $crate::print!("{}\n", format_args!($($arg)*)) };
}
