//! Nonblocking numeric IPv4 connect through the accepted libc-aros boundary.
use std::{io, net::{SocketAddr, TcpStream}, os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd}};
pub(super) fn set_nonblocking(fd: BorrowedFd<'_>) -> io::Result<()> {
    let mut enabled: i32 = 1;
    if unsafe { libc::ioctl(fd.as_raw_fd(), libc::FIONBIO, &mut enabled) } < 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}
pub(super) fn connect(addr: SocketAddr) -> io::Result<TcpStream> {
    let addr = match addr { SocketAddr::V4(addr) => addr, _ => return Err(io::ErrorKind::Unsupported.into()) };
    let raw = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    if raw < 0 { return Err(io::Error::last_os_error()); }
    let socket = unsafe { OwnedFd::from_raw_fd(raw) };
    if unsafe { libc::fcntl(raw, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 { return Err(io::Error::last_os_error()); }
    let stream = TcpStream::from(socket);
    stream.set_nonblocking(true)?;
    let address = libc::sockaddr_in {
        sin_len: std::mem::size_of::<libc::sockaddr_in>() as u8,
        sin_family: libc::AF_INET as _, sin_port: addr.port().to_be(),
        sin_addr: libc::in_addr { s_addr: u32::from_ne_bytes(addr.ip().octets()) }, sin_zero: [0; 8],
    };
    let result = unsafe { libc::connect(raw, (&address as *const libc::sockaddr_in).cast(), std::mem::size_of_val(&address) as _) };
    if result < 0 {
        let error = io::Error::last_os_error();
        if !matches!(error.raw_os_error(), Some(libc::EINPROGRESS | libc::EWOULDBLOCK)) { return Err(error); }
    }
    Ok(stream)
}
