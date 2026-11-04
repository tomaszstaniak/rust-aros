pub mod framing;

#[cfg(test)]
mod tests {
    use super::framing::{read_frame, write_frame, MAX_BODY, MAX_HEADER};
    use std::io::{BufReader, Cursor};

    #[test]
    fn fragmented_utf8_frames_round_trip_and_end_at_eof() {
        let body = "{\"text\":\"Zażółć\"}".as_bytes();
        let mut wire = Vec::new();
        write_frame(&mut wire, body).unwrap();
        write_frame(&mut wire, b"").unwrap();
        let mut reader = BufReader::with_capacity(1, Cursor::new(wire));
        assert_eq!(read_frame(&mut reader).unwrap().unwrap(), body);
        assert_eq!(read_frame(&mut reader).unwrap().unwrap(), b"");
        assert_eq!(read_frame(&mut reader).unwrap(), None);
    }

    #[test]
    fn additional_headers_and_case_insensitive_length_are_accepted() {
        let mut wire = Cursor::new(
            b"Content-Type: application/vscode-jsonrpc\r\ncontent-length: 2\r\n\r\n{}".as_slice(),
        );
        assert_eq!(read_frame(&mut wire).unwrap().unwrap(), b"{}");
    }

    #[test]
    fn ambiguous_and_malformed_headers_fail() {
        for wire in [
            "Content-Length: 1\r\nContent-Length: 2\r\n\r\n{}",
            "Content-Length: -1\r\n\r\n",
            "Content-Length: +1\r\n\r\nx",
            "Content-Length: x\r\n\r\n",
            "Content-Type: json\r\n\r\n",
            "Content-Length: 2\n\n{}",
        ] {
            assert!(
                read_frame(&mut Cursor::new(wire.as_bytes())).is_err(),
                "{wire:?}"
            );
        }
    }

    #[test]
    fn incomplete_header_or_body_is_not_clean_eof() {
        for wire in [
            "Content-Len",
            "Content-Length: 2\r\n",
            "Content-Length: 2\r\n\r\n{",
        ] {
            assert!(read_frame(&mut Cursor::new(wire.as_bytes())).is_err());
        }
    }

    #[test]
    fn body_and_header_limits_are_enforced() {
        let oversized = format!("Content-Length: {}\r\n\r\n", MAX_BODY + 1);
        assert!(read_frame(&mut Cursor::new(oversized)).is_err());
        let long_header = vec![b'a'; MAX_HEADER + 100];
        let mut reader = Cursor::new(long_header);
        assert!(read_frame(&mut reader).is_err());
        assert!(reader.position() <= (MAX_HEADER + 1) as u64);
        assert!(write_frame(&mut Vec::new(), &vec![0; MAX_BODY + 1]).is_err());
    }
}
