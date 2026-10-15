//! Bounded Content-Length framing for the process transport fixture.
use std::io::{self, BufRead, Read, Write};
pub const MAX_BODY: usize = 1024 * 1024;
pub const MAX_HEADER: usize = 8192;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut total = 0;
    let mut length = None;
    loop {
        let mut line = Vec::new();
        let n = (&mut *reader)
            .take((MAX_HEADER - total + 1) as u64)
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
        if total > MAX_HEADER {
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
            if n > MAX_BODY {
                return Err(invalid("body too large"));
            }
            length = Some(n);
        }
    }
    let length = length.ok_or_else(|| invalid("missing Content-Length"))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}
pub fn write_frame(writer: &mut impl Write, body: &[u8]) -> io::Result<()> {
    if body.len() > MAX_BODY {
        return Err(invalid("body too large"));
    }
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(body)?;
    writer.flush()
}
