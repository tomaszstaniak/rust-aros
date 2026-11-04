//! Terminal bindings for the x86_64 ABIv11 SDK. csrc/terminal_abi.h checks
//! every layout and value here against the SDK headers at build time.
//!
//! The SDK's tcsetattr maps ICANON to the console's DOS mode (SetMode 0/1)
//! and ignores every other flag; tcgetattr reports ICANON only and leaves
//! the rest of the caller's structure as it was.
use super::{c_char, c_int, c_uchar, c_uint, c_ulong, c_ushort};
#[cfg(feature = "rustc-dep-of-std")]
use core::prelude::v1::*;

pub type cc_t = c_uchar;
pub type tcflag_t = c_uint;
pub type speed_t = c_uint;
pub const NCCS: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct termios {
    pub c_iflag: tcflag_t,
    pub c_oflag: tcflag_t,
    pub c_cflag: tcflag_t,
    pub c_lflag: tcflag_t,
    pub c_cc: [cc_t; NCCS],
    pub internal: [c_char; 64],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct winsize {
    pub ws_row: c_ushort,
    pub ws_col: c_ushort,
    pub ws_xpixel: c_ushort,
    pub ws_ypixel: c_ushort,
}

pub const TCSANOW: c_int = 0;
pub const TCSADRAIN: c_int = 1;
pub const TCSAFLUSH: c_int = 2;
// Request values as unsigned long, the type the other targets give them.
pub const TIOCGWINSZ: c_ulong = 0x40087468;
pub const VEOF: usize = 0;
pub const VEOL: usize = 1;
pub const VERASE: usize = 2;
pub const VINTR: usize = 3;
pub const VKILL: usize = 4;
pub const VMIN: usize = 5;
pub const VQUIT: usize = 6;
pub const VSTART: usize = 7;
pub const VSTOP: usize = 8;
pub const VSUSP: usize = 9;
pub const VTIME: usize = 10;
pub const BRKINT: tcflag_t = 0o1;
pub const ICRNL: tcflag_t = 0o2;
pub const IGNBRK: tcflag_t = 0o4;
pub const IGNCR: tcflag_t = 0o10;
pub const IGNPAR: tcflag_t = 0o20;
pub const INLCR: tcflag_t = 0o40;
pub const INPCK: tcflag_t = 0o100;
pub const ISTRIP: tcflag_t = 0o200;
pub const IXANY: tcflag_t = 0o400;
pub const IXOFF: tcflag_t = 0o1000;
pub const IXON: tcflag_t = 0o2000;
pub const PARMRK: tcflag_t = 0o4000;
pub const OPOST: tcflag_t = 0o1;
pub const ONLCR: tcflag_t = 0o2;
pub const CSIZE: tcflag_t = 0o3;
pub const CS8: tcflag_t = 0o3;
pub const PARENB: tcflag_t = 0o20;
pub const ECHO: tcflag_t = 0o1;
pub const ECHOE: tcflag_t = 0o2;
pub const ECHOK: tcflag_t = 0o4;
pub const ECHONL: tcflag_t = 0o10;
pub const ICANON: tcflag_t = 0o20;
pub const IEXTEN: tcflag_t = 0o100;
pub const ISIG: tcflag_t = 0o200;

extern "C" {
    pub fn tcgetattr(fd: c_int, value: *mut termios) -> c_int;
    pub fn tcsetattr(fd: c_int, action: c_int, value: *const termios) -> c_int;
}

/// The SDK has no cfmakeraw. This is the BSD/glibc transformation, done
/// locally; applying it is still tcsetattr's job.
///
/// # Safety
/// `value` must point to an initialized, exclusively accessible termios.
pub unsafe extern "C" fn cfmakeraw(value: *mut termios) {
    let t = unsafe { &mut *value };
    t.c_iflag &= !(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
    t.c_oflag &= !OPOST;
    t.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
    t.c_cflag = (t.c_cflag & !(CSIZE | PARENB)) | CS8;
    t.c_cc[VMIN] = 1;
    t.c_cc[VTIME] = 0;
}
