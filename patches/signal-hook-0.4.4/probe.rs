use std::io::ErrorKind;
use std::net::UdpSocket;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let received = Arc::new(AtomicBool::new(false));
    let id = signal_hook::flag::register(libc::SIGUSR2, received.clone()).unwrap();
    signal_hook::low_level::raise(libc::SIGUSR2).unwrap();
    assert!(received.load(Ordering::SeqCst));
    assert!(signal_hook::low_level::unregister(id));

    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut buffer = [0u8; 16];
    let read = unsafe {
        libc::recv(
            socket.as_raw_fd(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            libc::MSG_DONTWAIT,
        )
    };
    assert_eq!(read, -1);
    assert_eq!(
        std::io::Error::last_os_error().kind(),
        ErrorKind::WouldBlock
    );
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    sender
        .send_to(b"wake", socket.local_addr().unwrap())
        .unwrap();
    for _ in 0..100 {
        let read = unsafe {
            libc::recv(
                socket.as_raw_fd(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                libc::MSG_DONTWAIT,
            )
        };
        if read >= 0 {
            assert_eq!(read, 4);
            assert_eq!(&buffer[..4], b"wake");
            println!("ABIV1_SIGNAL_HOOK_OK");
            return;
        }
        assert_eq!(
            std::io::Error::last_os_error().kind(),
            ErrorKind::WouldBlock
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("UDP loopback packet was not delivered within the retry window");
}
