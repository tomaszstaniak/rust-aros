use crate::Presentation;
use core::{
    fmt::{self, Write},
    marker::PhantomData,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryStatus {
    Supported,
    NotInteractive,
    Unsupported,
    InvalidReply,
    Failed,
    BadArgument,
}

extern "C" {
    fn rs_limpet_current(
        handle: *mut usize,
        presentation: *mut Presentation,
        error: *mut i64,
    ) -> i32;
    fn rs_limpet_write(handle: usize, bytes: *const u8, len: usize) -> i32;
}

/// A task-local view of the process's current DOS output.
///
/// Keep its process-owned handle open and unchanged while using this object.
/// Flush buffered Rust/C stdout before querying. The synchronous query does
/// not read the keyboard or change modes; DOS does not provide a timeout.
/// Ordinary files and unsupported consoles receive plain text.
pub struct Output {
    handle: usize,
    presentation: Presentation,
    status: QueryStatus,
    io_error: i64,
    // DOS handles are not used from a second task through this wrapper.
    _task: PhantomData<*mut ()>,
}
impl Output {
    /// Capture and query the process-owned current output handle.
    ///
    /// # Safety
    /// The caller must keep that handle valid and unchanged until this object
    /// is dropped, and prevent its use by other tasks during these calls.
    /// Separate tasks can independently capture the same inherited handle;
    /// the task-local marker alone cannot enforce exclusive handle access.
    pub unsafe fn current() -> Self {
        let mut result = Self {
            handle: 0,
            presentation: Presentation::default(),
            status: QueryStatus::Failed,
            io_error: 0,
            _task: PhantomData,
        };
        let status = unsafe {
            rs_limpet_current(
                &mut result.handle,
                &mut result.presentation,
                &mut result.io_error,
            )
        };
        result.status = match status {
            0 => QueryStatus::Supported,
            1 => QueryStatus::NotInteractive,
            2 => QueryStatus::Unsupported,
            3 => QueryStatus::InvalidReply,
            5 => QueryStatus::BadArgument,
            _ => QueryStatus::Failed,
        };
        if status != 0 {
            result.presentation = Presentation::default();
        }
        result
    }
    pub fn status(&self) -> QueryStatus {
        self.status
    }
    pub fn io_error(&self) -> i64 {
        self.io_error
    }
    pub fn capabilities(&self) -> Presentation {
        self.presentation
    }
    pub fn set_ansi(&mut self, index: u32, background: bool) -> fmt::Result {
        let sequence = self.presentation.ansi(index, background);
        self.write_str(sequence.as_str())
    }
    pub fn reset_colors(&mut self) -> fmt::Result {
        if !self.presentation.ansi(0, false).as_str().is_empty() {
            self.write_str("\x1b[39m\x1b[49m")?;
        }
        Ok(())
    }
    pub fn set_title(&mut self, title: &str) -> fmt::Result {
        let sequence = self.presentation.title(title);
        self.write_str(sequence.as_str())
    }
    /// Rewrite one line when confirmed; otherwise print an ordinary line.
    pub fn progress(&mut self, text: &str) -> fmt::Result {
        if self.presentation.can_rewrite_line() {
            self.write_str("\r")?;
            self.write_str(text)?;
            self.write_str("\x1b[K")
        } else {
            writeln!(self, "{text}")
        }
    }
}
impl Write for Output {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.is_empty() {
            return Ok(());
        }
        if self.handle == 0
            || unsafe { rs_limpet_write(self.handle, text.as_ptr(), text.len()) } != 0
        {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}
