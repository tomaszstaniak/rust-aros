//! Allocation-free Limpet SDK bindings, usable from std and no_std programs.
#![no_std]
#[cfg(test)]
extern crate std;

use core::ffi::c_char;

// limpet/tty_control.h: XTY_PRESENT_KNOWN_INPUT and the XTY_INPUT_* bits.
const KNOWN_INPUT: u32 = 32;
const INPUT_BRACKETED_PASTE: u32 = 1;
const INPUT_RESIZE_REPORT: u32 = 2;
const INPUT_WHEEL_REPORT: u32 = 4;
const INPUT_MODIFIED_ENTER: u32 = 8;

/// Validated SDK 0.3 snapshot. Unknown capabilities remain disabled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Presentation {
    known: u32,
    colors: u32,
    styles: u32,
    lines: u32,
    rows: u32,
    columns: u32,
    interpretation: u32,
    input: u32,
    features: u32,
}
extern "C" {
    fn limpet_sgr_ansi(
        buf: *mut c_char,
        size: usize,
        p: *const Presentation,
        index: u32,
        background: i32,
    ) -> usize;
    fn limpet_sgr_rgb(
        buf: *mut c_char,
        size: usize,
        p: *const Presentation,
        r: u32,
        g: u32,
        b: u32,
        background: i32,
    ) -> usize;
    fn limpet_osc_title(
        buf: *mut c_char,
        size: usize,
        p: *const Presentation,
        text: *const c_char,
    ) -> usize;
    fn limpet_has_bold(p: *const Presentation) -> i32;
    fn limpet_can_rewrite_line(p: *const Presentation) -> i32;
    fn limpet_columns(p: *const Presentation) -> u32;
    fn limpet_rows(p: *const Presentation) -> u32;
    #[cfg(test)]
    fn limpet_presentation_decode_v3(reply: *const u32, p: *mut Presentation) -> i32;
}
/// A complete terminal sequence, or an empty sequence for unsupported input.
pub struct Sequence {
    bytes: [u8; 264],
    len: usize,
}
impl Sequence {
    fn build(write: impl FnOnce(*mut c_char, usize) -> usize) -> Self {
        let mut sequence = Self {
            bytes: [0; 264],
            len: 0,
        };
        let len = write(sequence.bytes.as_mut_ptr().cast(), sequence.bytes.len());
        if len < sequence.bytes.len() {
            sequence.len = len;
        }
        sequence
    }
    pub fn as_str(&self) -> &str {
        // SGR is ASCII and the SDK title builder validates UTF-8.
        unsafe { core::str::from_utf8_unchecked(&self.bytes[..self.len]) }
    }
}
impl Presentation {
    pub fn ansi(&self, index: u32, background: bool) -> Sequence {
        Sequence::build(|b, n| unsafe { limpet_sgr_ansi(b, n, self, index, background as i32) })
    }
    pub fn rgb(&self, red: u8, green: u8, blue: u8, background: bool) -> Sequence {
        Sequence::build(|b, n| unsafe {
            limpet_sgr_rgb(
                b,
                n,
                self,
                red as u32,
                green as u32,
                blue as u32,
                background as i32,
            )
        })
    }
    pub fn title(&self, title: &str) -> Sequence {
        let mut text = [0u8; 256];
        if title.len() > 255 || title.as_bytes().contains(&0) {
            return Sequence::build(|_, _| 0);
        }
        text[..title.len()].copy_from_slice(title.as_bytes());
        Sequence::build(|b, n| unsafe { limpet_osc_title(b, n, self, text.as_ptr().cast()) })
    }
    /// (columns, rows), valid at query time; not an automatic resize subscription.
    pub fn geometry(&self) -> Option<(u32, u32)> {
        let columns = unsafe { limpet_columns(self) };
        let rows = unsafe { limpet_rows(self) };
        if columns > 0 && rows > 0 {
            Some((columns, rows))
        } else {
            None
        }
    }
    pub fn has_bold(&self) -> bool {
        unsafe { limpet_has_bold(self) != 0 }
    }
    pub fn can_rewrite_line(&self) -> bool {
        unsafe { limpet_can_rewrite_line(self) != 0 }
    }
    /// Input modes a raw-mode program may enable, valid only when the
    /// terminal reported its input set (known bit 32). Limpet 0.1.0 and
    /// CON: report none.
    fn input(&self, bit: u32) -> bool {
        self.known & KNOWN_INPUT != 0 && self.input & bit != 0
    }
    /// Bracketed paste, `CSI ? 2004 h` (Limpet 0.2.0 and later).
    pub fn has_bracketed_paste(&self) -> bool {
        self.input(INPUT_BRACKETED_PASTE)
    }
    /// Resize reports, `CSI ? 2048 h`: `CSI 48 ; rows ; columns ; height ; width t`
    /// on the input stream after every size change (Limpet 0.2.0 and later).
    pub fn has_resize_reports(&self) -> bool {
        self.input(INPUT_RESIZE_REPORT)
    }
    /// Wheel reports, `CSI ? 2100 h`, a Limpet extension.
    pub fn has_wheel_reports(&self) -> bool {
        self.input(INPUT_WHEEL_REPORT)
    }
    /// Shift+Enter as `CSI 27 ; 2 ; 13 ~` after `CSI > 4 ; 1 m` (Limpet 0.6.0,
    /// SDK 0.4.0 bit 8; the binary layout is the same as SDK 0.3.0's).
    pub fn has_modified_enter(&self) -> bool {
        self.input(INPUT_MODIFIED_ENTER)
    }
    #[cfg(test)]
    fn decode(reply: &[u32; 16]) -> Result<Self, ()> {
        let mut p = Self::default();
        if unsafe { limpet_presentation_decode_v3(reply.as_ptr(), &mut p) } == 0 {
            Ok(p)
        } else {
            Err(())
        }
    }
}
#[cfg(any(test, target_os = "aros"))]
mod output;
#[cfg(any(test, target_os = "aros"))]
pub use output::{Output, QueryStatus};

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(known: u32, colors: u32, interpretation: u32, features: u32) -> Presentation {
        let mut words = [0u32; 16];
        words[0] = 1;
        words[1] = 13;
        words[2] = 0x58545931;
        words[4] = known;
        words[5] = colors;
        words[6] = 1;
        words[7] = 3;
        words[8] = 24;
        words[9] = 80;
        words[10] = interpretation;
        words[12] = features;
        Presentation::decode(&words).unwrap()
    }

    #[test]
    fn input_modes_need_the_known_input_bit() {
        let mut words = [0u32; 16];
        words[0] = 1;
        words[1] = 13;
        words[2] = 0x58545931;
        words[11] = 1 | 2 | 4 | 8;
        let unknown = Presentation::decode(&words).unwrap();
        assert!(!unknown.has_resize_reports() && !unknown.has_bracketed_paste());
        assert!(!unknown.has_wheel_reports() && !unknown.has_modified_enter());
        words[4] = 32;
        let known = Presentation::decode(&words).unwrap();
        assert!(known.has_resize_reports() && known.has_bracketed_paste());
        assert!(known.has_wheel_reports() && known.has_modified_enter());
        words[11] = 2;
        let resize = Presentation::decode(&words).unwrap();
        assert!(resize.has_resize_reports() && !resize.has_modified_enter());
    }

    #[test]
    fn unknown_fields_never_enable_output_sequences() {
        let p = reply(0, 3, 1, 1);
        assert_eq!(p.ansi(2, false).as_str(), "");
        assert_eq!(p.title("hello").as_str(), "");
        assert_eq!(p.geometry(), None);
        assert!(!p.can_rewrite_line());
    }

    #[test]
    fn known_pens_use_indexed_ansi_colors() {
        let p = reply(1 | 16, 3, 1, 0);
        assert_eq!(p.ansi(2, false).as_str(), "\x1b[38;5;2m");
        assert_eq!(p.ansi(15, true).as_str(), "\x1b[48;5;15m");
        assert_eq!(p.ansi(16, false).as_str(), "");
        assert_eq!(reply(1, 1, 0, 0).ansi(2, false).as_str(), "\x1b[32m");
        assert_eq!(reply(1, 99, 0, 0).ansi(2, false).as_str(), "");
        assert_eq!(
            reply(1, 3, 0, 0).rgb(255, 0, 0, false).as_str(),
            "\x1b[38;2;255;0;0m"
        );
        assert_eq!(
            reply(1, 2, 0, 0).rgb(255, 0, 0, false).as_str(),
            "\x1b[38;5;196m"
        );
        assert_eq!(reply(1, 1, 0, 0).rgb(255, 0, 0, false).as_str(), "");
    }

    #[test]
    fn title_requires_capability_and_rejects_embedded_controls() {
        let p = reply(64, 0, 0, 1);
        assert_eq!(p.title("Zażółć").as_str(), "\x1b]2;Zażółć\x1b\\");
        assert_eq!(p.title("").as_str(), "\x1b]2;\x1b\\");
        for text in [
            "hello\0hidden",
            "hello\nworld",
            "bad\x1btitle",
            "bad\u{85}title",
        ] {
            assert_eq!(p.title(text).as_str(), "");
        }
        assert_eq!(reply(0, 0, 0, 1).title("hello").as_str(), "");
        assert_eq!(p.title(&"x".repeat(255)).as_str().len(), 261);
        assert_eq!(p.title(&"x".repeat(256)).as_str(), "");
        assert!(!p.title(&"é".repeat(127)).as_str().is_empty());
        assert_eq!(p.title(&"é".repeat(128)).as_str(), "");
    }

    #[test]
    fn geometry_and_line_controls_use_known_bits() {
        let p = reply(8 | 4 | 2, 0, 0, 0);
        assert_eq!(p.geometry(), Some((80, 24)));
        assert!(p.can_rewrite_line());
        assert!(p.has_bold());
    }

    #[test]
    fn invalid_reply_cannot_become_capabilities() {
        assert!(Presentation::decode(&[0; 16]).is_err());
        let mut words = [0; 16];
        words[0] = 1;
        words[1] = 13;
        words[2] = 0x58545931;
        words[4] = 8; // Geometry marked known but zero dimensions.
        assert!(Presentation::decode(&words).is_err());
    }
    static IO: std::sync::Mutex<(i32, Presentation, std::vec::Vec<u8>, bool)> =
        std::sync::Mutex::new((
            1,
            Presentation {
                known: 0,
                colors: 0,
                styles: 0,
                lines: 0,
                rows: 0,
                columns: 0,
                interpretation: 0,
                input: 0,
                features: 0,
            },
            std::vec::Vec::new(),
            false,
        ));
    #[no_mangle]
    unsafe extern "C" fn rs_limpet_current(
        handle: *mut usize,
        p: *mut Presentation,
        error: *mut i64,
    ) -> i32 {
        let state = IO.lock().unwrap();
        unsafe {
            *handle = 42;
            *p = state.1;
            *error = 209;
        }
        state.0
    }
    #[no_mangle]
    unsafe extern "C" fn rs_limpet_write(handle: usize, bytes: *const u8, len: usize) -> i32 {
        assert_eq!(handle, 42);
        let mut state = IO.lock().unwrap();
        if state.3 {
            return 1;
        }
        state
            .2
            .extend_from_slice(unsafe { core::slice::from_raw_parts(bytes, len) });
        0
    }
    #[test]
    fn output_falls_back_to_plain_text_and_propagates_write_failures() {
        use core::fmt::Write;
        let supported = reply(1 | 4 | 8 | 16 | 64, 3, 1, 1);
        *IO.lock().unwrap() = (1, supported, std::vec::Vec::new(), false);
        let mut out = unsafe { Output::current() };
        assert_eq!(out.status(), QueryStatus::NotInteractive);
        out.set_ansi(2, false).unwrap();
        out.set_title("hidden").unwrap();
        out.progress("hello").unwrap();
        out.reset_colors().unwrap();
        assert_eq!(IO.lock().unwrap().2, b"hello\n");
        *IO.lock().unwrap() = (0, supported, std::vec::Vec::new(), false);
        let mut out = unsafe { Output::current() };
        assert_eq!(out.status(), QueryStatus::Supported);
        assert_eq!(out.capabilities().geometry(), Some((80, 24)));
        out.set_ansi(2, false).unwrap();
        out.progress("working").unwrap();
        assert_eq!(IO.lock().unwrap().2, b"\x1b[38;5;2m\rworking\x1b[K");
        IO.lock().unwrap().3 = true;
        assert!(out.write_str("cannot write").is_err());
        for (code, status) in [
            (2, QueryStatus::Unsupported),
            (3, QueryStatus::InvalidReply),
            (4, QueryStatus::Failed),
            (5, QueryStatus::BadArgument),
            (77, QueryStatus::Failed),
        ] {
            *IO.lock().unwrap() = (code, supported, std::vec::Vec::new(), false);
            let mut out = unsafe { Output::current() };
            assert_eq!(out.status(), status);
            assert_eq!(out.capabilities().geometry(), None);
            out.set_title("hidden").unwrap();
            out.set_ansi(2, false).unwrap();
            assert!(IO.lock().unwrap().2.is_empty());
        }
    }
}
