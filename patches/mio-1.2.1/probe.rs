use mio::{net::TcpListener, Events, Interest, Poll, Token, Waker};
use std::{io::{ErrorKind, Write}, net::TcpStream, time::Duration};

fn main() -> std::io::Result<()> {
    let mut poll = Poll::new()?;
    let mut listener = TcpListener::bind("127.0.0.1:0".parse().unwrap())?;
    let addr = listener.local_addr()?;
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
    poll.registry().register(&mut listener, Token(0), Interest::READABLE)?;
    let mut client = TcpStream::connect(addr)?;
    let mut events = Events::with_capacity(4);
    poll.poll(&mut events, Some(Duration::from_secs(1)))?;
    assert!(events.iter().any(|e| e.token() == Token(0) && e.is_readable()));
    poll.poll(&mut events, Some(Duration::from_millis(20)))?;
    assert!(events.is_empty());
    poll.registry().deregister(&mut listener)?;
    let (_stream, _) = listener.accept()?;
    client.write_all(b"MIO")?;
    let waker = Waker::new(poll.registry(), Token(1))?;
    waker.wake()?;
    poll.poll(&mut events, Some(Duration::from_secs(1)))?;
    assert!(events.iter().any(|e| e.token() == Token(1)));
    println!("MIO_AROS_OK");
    Ok(())
}
