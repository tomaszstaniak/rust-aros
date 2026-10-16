//! Socket/file namespace regression gate. Run only against fixture.py.
use std::io::{IoSlice, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::fd::{AsRawFd, FromRawFd};
use std::time::{Duration, Instant};

fn ready(stream: &TcpStream, events: i16, timeout: i32) -> i16 {
    let mut fd = libc::pollfd { fd: stream.as_raw_fd(), events, revents: 0 };
    assert_eq!(unsafe { libc::poll(&mut fd, 1, timeout) }, 1, "socket readiness timed out");
    fd.revents
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint: SocketAddr = std::env::args().nth(1).ok_or("expected numeric endpoint")?.parse()?;
    let stdin_before = unsafe { libc::fcntl(0, libc::F_GETFD) };
    let mut stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))?;
    println!("CONNECTED fd={} stdin_flags={stdin_before}", stream.as_raw_fd());
    stream.set_nonblocking(true)?;
    let mut byte = [0];
    assert_eq!(stream.read(&mut byte).unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    println!("PASS pending read is WouldBlock");
    assert_ne!(ready(&stream, libc::POLLOUT, 1000) & libc::POLLOUT, 0);
    let mut fd = libc::pollfd { fd: stream.as_raw_fd(), events: libc::POLLIN, revents: -1 };
    let start = Instant::now();
    assert_eq!(unsafe { libc::poll(&mut fd, 1, 20) }, 0);
    assert_eq!(fd.revents, 0);
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(unsafe { libc::ioctl(stream.as_raw_fd(), 0x20006601) }, 0); // SDK FIOCLEX, no vararg.
    assert_eq!(unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFD) }, libc::FD_CLOEXEC);
    let mut unsupported = libc::pollfd { fd: stream.as_raw_fd(), events: 2, revents: 0 };
    assert_eq!(unsafe { libc::poll(&mut unsupported, 1, 0) }, -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ENOTSUP));
    let parts = [IoSlice::new(b"delayed"), IoSlice::new(b"\n")];
    let sent = stream.write_vectored(&parts)?;
    assert!(sent > 0 && sent <= 8);
    stream.write_all(&b"delayed\n"[sent..])?;
    assert_ne!(ready(&stream, libc::POLLIN, 2000) & libc::POLLIN, 0);
    assert_eq!(stream.read(&mut byte)?, 1);
    assert_eq!(byte[0], b'R');
    assert_ne!(ready(&stream, libc::POLLIN, 2000) & libc::POLLIN, 0);
    assert_eq!(stream.read(&mut byte)?, 0);
    let old = stream.as_raw_fd();
    drop(stream);
    fd.fd = old;
    assert_eq!(unsafe { libc::poll(&mut fd, 1, 1000) }, 1);
    assert_eq!(fd.revents, 0x20); // POLLNVAL, never pass a tag to FD_SET.
    fd.fd = i32::MAX;
    assert_eq!(unsafe { libc::poll(&mut fd, 1, 1000) }, 1);
    assert_eq!(fd.revents, 0x20);
    println!("PASS writable, deadline, vectored write, delayed read, EOF, invalid descriptors");

    // More lifecycles than the small BSD descriptor table can hide as leaks.
    for _ in 0..80 {
        let mut stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))?;
        stream.set_nonblocking(true)?;
        stream.write_all(b"fast\n")?;
        ready(&stream, libc::POLLIN, 2000);
        assert_eq!(stream.read(&mut byte)?, 1);
        assert_eq!(byte[0], b'R');
    }
    assert_eq!(unsafe { libc::fcntl(0, libc::F_GETFD) }, stdin_before, "socket drop changed stdin");
    let path = std::path::PathBuf::from(format!("RAM:rust-net-fd-{}", std::process::id()));
    std::fs::write(&path, b"file namespace intact")?;
    let mut file = std::fs::File::open(&path)?;
    // This old SDK declares F_DUPFD_CLOEXEC but its fcntl does not implement
    // it. Keep that pre-existing std File::try_clone gap visible separately.
    assert_eq!(file.try_clone().unwrap_err().raw_os_error(), Some(libc::EINVAL));
    let duplicate = unsafe { libc::dup(file.as_raw_fd()) };
    assert!(duplicate >= 0, "ordinary DOS dup failed");
    let mut clone = unsafe { std::fs::File::from_raw_fd(duplicate) };
    let mut text = String::new();
    clone.read_to_string(&mut text)?;
    assert_eq!(text, "file namespace intact");
    assert_eq!(file.read(&mut byte)?, 0);
    drop(clone); drop(file); std::fs::remove_file(path)?;
    println!("PASS 80 socket lifecycles, stdin preserved, ordinary file read/write/dup/drop");

    let mut stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))?;
    stream.set_nonblocking(true)?;
    let port = endpoint.port();
    std::thread::spawn(move || {
        let dns_error = TcpStream::connect(("localhost", port)).unwrap_err();
        assert_eq!(dns_error.raw_os_error(), Some(libc::ENOTSUP));
        let error = stream.read(&mut [0]).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOTSUP));
        // Drop on a foreign task queues the close. The next owner operation drains it.
    }).join().unwrap();
    let stream = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))?;
    drop(stream);
    println!("PASS foreign-task I/O refused, foreign drop followed by owner cleanup");
    println!("PASS socket boundary acceptance");
    Ok(())
}
