use socket2::{Domain, MsgHdr, MsgHdrMut, Protocol, Socket, Type};
use std::{io::{ErrorKind, IoSlice}, mem::MaybeUninit, net::SocketAddr, time::Duration};

const _: () = {
    use std::mem::{align_of, offset_of, size_of};
    assert!(size_of::<libc::msghdr>() == 48 && align_of::<libc::msghdr>() == 8);
    assert!(offset_of!(libc::msghdr, msg_namelen) == 8);
    assert!(offset_of!(libc::msghdr, msg_iov) == 16);
    assert!(offset_of!(libc::msghdr, msg_iovlen) == 24);
    assert!(offset_of!(libc::msghdr, msg_control) == 32);
    assert!(offset_of!(libc::msghdr, msg_controllen) == 40);
    assert!(offset_of!(libc::msghdr, msg_flags) == 44);
    assert!(size_of::<libc::ip_mreq_source>() == 12);
    assert!(offset_of!(libc::ip_mreq_source, imr_sourceaddr) == 4);
    assert!(offset_of!(libc::ip_mreq_source, imr_interface) == 8);
};

fn main() -> std::io::Result<()> {
    let loopback = SocketAddr::from(([127, 0, 0, 1], 0)).into();
    let receiver = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    receiver.set_reuse_address(true)?;
    assert!(receiver.reuse_address()?);
    receiver.bind(&loopback)?;
    receiver.set_read_timeout(Some(Duration::from_secs(2)))?;
    let addr = receiver.local_addr()?;
    let sender = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    assert_eq!(sender.send_to(b"socket2", &addr)?, 7);
    let mut data = [MaybeUninit::uninit(); 32];
    let (read, from) = receiver.recv_from(&mut data)?;
    assert_eq!(read, 7);
    assert!(from.as_socket_ipv4().unwrap().ip().is_loopback());
    let packet = unsafe { std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), read) };
    assert_eq!(packet, b"socket2");

    let slices = [IoSlice::new(b"ignored")];
    let msg = MsgHdr::new().with_addr(&addr).with_buffers(&slices);
    assert_eq!(sender.sendmsg(&msg, 0).unwrap_err().kind(), ErrorKind::Unsupported);
    let mut msg = MsgHdrMut::new();
    assert_eq!(receiver.recvmsg(&mut msg, 0).unwrap_err().kind(), ErrorKind::Unsupported);
    println!("SOCKET2_AROS_OK");
    Ok(())
}
