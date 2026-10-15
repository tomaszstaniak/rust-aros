//! Bounded blocking Content-Length framing.
//!
//! DOS pipe headers require single-byte reads. Pass an unbuffered source:
//! wrapping an already buffered stdin cannot change its underlying read size.
use std::io::{self, BufRead, BufReader, Read, Write};
pub const MAX_BODY: usize = 1024 * 1024;
pub const MAX_HEADER: usize = 8192;
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    header: usize,
    body: usize,
}
impl Limits {
    pub fn new(header: usize, body: usize) -> io::Result<Self> {
        if header == 0 || header == usize::MAX || body > isize::MAX as usize {
            return Err(invalid("invalid transport limits"));
        }
        Ok(Self { header, body })
    }
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            header: MAX_HEADER,
            body: MAX_BODY,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum ReadPolicy {
    Buffered,
    DosPipe,
}
pub struct FrameReader<R> {
    reader: BufReader<R>,
    limits: Limits,
    failed: bool,
    eof: bool,
}
impl<R: Read> FrameReader<R> {
    pub fn new(reader: R, policy: ReadPolicy) -> Self {
        Self::with_limits(reader, policy, Limits::default())
    }
    pub fn with_limits(reader: R, policy: ReadPolicy, limits: Limits) -> Self {
        let capacity = match policy {
            ReadPolicy::Buffered => 8192,
            ReadPolicy::DosPipe => 1,
        };
        Self {
            reader: BufReader::with_capacity(capacity, reader),
            limits,
            failed: false,
            eof: false,
        }
    }
    pub fn read_frame(&mut self) -> io::Result<Option<Vec<u8>>> {
        if self.failed {
            return Err(invalid("reader has failed"));
        }
        if self.eof {
            return Ok(None);
        }
        let result = read_frame_with_limits(&mut self.reader, self.limits);
        match &result {
            Err(_) => self.failed = true,
            Ok(None) => self.eof = true,
            _ => {}
        }
        result
    }
}
pub struct FrameWriter<W> {
    writer: W,
    limits: Limits,
    failed: bool,
}
impl<W: Write> FrameWriter<W> {
    pub fn new(writer: W) -> Self {
        Self::with_limits(writer, Limits::default())
    }
    pub fn with_limits(writer: W, limits: Limits) -> Self {
        Self {
            writer,
            limits,
            failed: false,
        }
    }
    pub fn write_frame(&mut self, body: &[u8]) -> io::Result<()> {
        if self.failed {
            return Err(invalid("writer has failed"));
        }
        let result = write_frame_with_limits(&mut self.writer, body, self.limits);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
/// Stateless compatibility entry point; callers must stop after any error.
pub fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    read_frame_with_limits(reader, Limits::default())
}
/// Stateless compatibility entry point; callers must stop after any error.
pub fn write_frame(writer: &mut impl Write, body: &[u8]) -> io::Result<()> {
    write_frame_with_limits(writer, body, Limits::default())
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn read_frame_with_limits(
    reader: &mut impl BufRead,
    limits: Limits,
) -> io::Result<Option<Vec<u8>>> {
    let mut total = 0;
    let mut length = None;
    loop {
        let mut line = Vec::new();
        let n = (&mut *reader)
            .take((limits.header - total + 1) as u64)
            .read_until(b'\n', &mut line)?;
        if n == 0 {
            return if total == 0 {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "partial frame header",
                ))
            };
        }
        total += n;
        if total > limits.header {
            return Err(invalid("header too large"));
        }
        if !line.ends_with(b"\r\n") {
            return Err(invalid("header requires CRLF"));
        }
        if line == b"\r\n" {
            break;
        }
        let text = std::str::from_utf8(&line[..line.len() - 2])
            .map_err(|_| invalid("header is not UTF-8"))?;
        let (name, value) = text
            .split_once(':')
            .ok_or_else(|| invalid("malformed header"))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err(invalid("duplicate Content-Length"));
            }
            let value = value.trim();
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid("invalid Content-Length"));
            }
            let n = value
                .parse::<usize>()
                .map_err(|_| invalid("invalid Content-Length"))?;
            if n > limits.body {
                return Err(invalid("body too large"));
            }
            length = Some(n);
        }
    }
    let length = length.ok_or_else(|| invalid("missing Content-Length"))?;
    let mut body = Vec::new();
    body.try_reserve_exact(length)
        .map_err(|_| invalid("body allocation failed"))?;
    body.resize(length, 0);
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}
fn write_frame_with_limits(writer: &mut impl Write, body: &[u8], limits: Limits) -> io::Result<()> {
    if body.len() > limits.body {
        return Err(invalid("body too large"));
    }
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    if header.len() > limits.header {
        return Err(invalid("header too large"));
    }
    writer.write_all(header.as_bytes())?;
    writer.write_all(body)?;
    writer.flush()
}
