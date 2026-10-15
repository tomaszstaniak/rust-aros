use aros_lsp_transport::{FrameReader, FrameWriter, Limits, ReadPolicy};
use std::{
    cell::Cell,
    io::{self, Cursor, Read, Write},
    rc::Rc,
};

struct ExactPipe {
    bytes: Cursor<Vec<u8>>,
    calls: Rc<Cell<usize>>,
}
impl Read for ExactPipe {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.calls.set(self.calls.get() + 1);
        let available = self.bytes.get_ref().len() - self.bytes.position() as usize;
        // Model an open DOS pipe: oversized reads cannot finish, even when
        // a complete small protocol message is already available.
        if out.len() > available {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.bytes.read(out)
    }
}
#[test]
fn dos_policy_receives_a_message_before_eof_without_an_oversized_read() {
    let bytes = b"Content-Length: 2\r\n\r\n{}".to_vec();
    let calls = Rc::new(Cell::new(0));
    let mut reader = FrameReader::new(
        ExactPipe {
            bytes: Cursor::new(bytes.clone()),
            calls: calls.clone(),
        },
        ReadPolicy::DosPipe,
    );
    assert_eq!(reader.read_frame().unwrap().unwrap(), b"{}");
    assert!(calls.get() > 1);
    let mut buffered = FrameReader::new(
        ExactPipe {
            bytes: Cursor::new(bytes),
            calls,
        },
        ReadPolicy::Buffered,
    );
    assert_eq!(
        buffered.read_frame().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}
#[test]
fn coalesced_utf8_and_zero_length_frames_preserve_boundaries_and_clean_eof() {
    let bytes = "Content-Length: 4\r\n\r\nżółContent-Length: 0\r\n\r\n";
    // Length counts bytes, including multibyte UTF-8 characters.
    let bytes = bytes.as_bytes();
    let first_end = b"Content-Length: 4\r\n\r\n".len() + 4;
    let mut wire = bytes[..first_end].to_vec();
    wire.extend_from_slice(b"Content-Length: 0\r\n\r\n");
    for policy in [ReadPolicy::Buffered, ReadPolicy::DosPipe] {
        let mut reader = FrameReader::new(Cursor::new(wire.clone()), policy);
        assert_eq!(reader.read_frame().unwrap().unwrap(), "żó".as_bytes());
        assert_eq!(reader.read_frame().unwrap().unwrap(), b"");
        assert_eq!(reader.read_frame().unwrap(), None);
        assert_eq!(reader.read_frame().unwrap(), None);
    }
}
#[test]
fn invalid_and_truncated_frames_poison_the_reader() {
    for wire in [
        "Content-Length: +1\r\n\r\nx",
        "Content-Length: 1\r\nContent-Length: 1\r\n\r\nx",
        "Content-Length: 1\n\nx",
        "Content-Length: 2\r\n\r\nx",
        "Content-Length: 1\r\n",
    ] {
        let mut reader = FrameReader::new(Cursor::new(wire), ReadPolicy::DosPipe);
        assert!(reader.read_frame().is_err(), "{wire:?}");
        assert_eq!(
            reader.read_frame().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
#[test]
fn explicit_limits_reject_oversized_input_before_body_read() {
    let limits = Limits::new(64, 3).unwrap();
    let mut reader = FrameReader::with_limits(
        Cursor::new("Content-Length: 4\r\n\r\nbody"),
        ReadPolicy::DosPipe,
        limits,
    );
    assert_eq!(
        reader.read_frame().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    let mut reader = FrameReader::with_limits(
        Cursor::new(vec![b'x'; 200]),
        ReadPolicy::DosPipe,
        Limits::new(32, 3).unwrap(),
    );
    assert_eq!(
        reader.read_frame().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(Limits::new(0, 1).is_err());
    assert!(Limits::new(usize::MAX, 1).is_err());
}
struct PartialWriter {
    bytes: Vec<u8>,
    flushes: Rc<Cell<usize>>,
    fail_after: usize,
    calls: Rc<Cell<usize>>,
}
impl Write for PartialWriter {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        self.calls.set(self.calls.get() + 1);
        if self.bytes.len() >= self.fail_after {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let n = input.len().min(2).min(self.fail_after - self.bytes.len());
        self.bytes.extend_from_slice(&input[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes.set(self.flushes.get() + 1);
        Ok(())
    }
}
#[test]
fn partial_writes_are_completed_and_each_frame_is_flushed() {
    let flushes = Rc::new(Cell::new(0));
    let bytes = Rc::new(std::cell::RefCell::new(Vec::new()));
    struct Sink(Rc<std::cell::RefCell<Vec<u8>>>, Rc<Cell<usize>>);
    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            let n = b.len().min(2);
            self.0.borrow_mut().extend_from_slice(&b[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.1.set(self.1.get() + 1);
            Ok(())
        }
    }
    let mut writer = FrameWriter::new(Sink(bytes.clone(), flushes.clone()));
    writer.write_frame("ż".as_bytes()).unwrap();
    writer.write_frame(b"").unwrap();
    assert_eq!(
        &*bytes.borrow(),
        "Content-Length: 2\r\n\r\nżContent-Length: 0\r\n\r\n".as_bytes()
    );
    assert_eq!(flushes.get(), 2);
}
#[test]
fn failed_write_is_terminal_and_never_appends_another_frame() {
    let calls = Rc::new(Cell::new(0));
    let mut writer = FrameWriter::new(PartialWriter {
        bytes: Vec::new(),
        flushes: Rc::new(Cell::new(0)),
        fail_after: 3,
        calls: calls.clone(),
    });
    assert_eq!(
        writer.write_frame(b"{}").unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    let count = calls.get();
    assert_eq!(
        writer.write_frame(b"{}").unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(calls.get(), count);
}

#[test]
fn writer_enforces_the_same_header_limit_as_reader() {
    let mut writer = FrameWriter::with_limits(Vec::new(), Limits::new(1, 3).unwrap());
    assert_eq!(
        writer.write_frame(b"{}").unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}
