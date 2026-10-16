use futures_lite::{future, io::{AsyncReadExt, AsyncWriteExt}};
use std::{os::fd::{AsRawFd, BorrowedFd}, num::NonZeroUsize, future::Future, io::{self, Read, Write}, net::{SocketAddr, TcpStream}, pin::Pin, sync::{Arc, Mutex, mpsc}, task::{Context, Poll, Waker}, time::{Duration, Instant}};

fn selector_gate(endpoint: SocketAddr) -> io::Result<()> {
    use polling::{Event, Events, Poller, PollMode};
    for _ in 0..80 { drop(Poller::new()?); }
    let poller = Arc::new(Poller::new()?);
    let mut events = Events::new();
    let (send, receive) = mpsc::channel::<bool>();
    let (ack, acknowledged) = mpsc::channel();
    let remote = poller.clone();
    let worker = std::thread::spawn(move || {
        while let Ok(delay) = receive.recv() {
            if delay { std::thread::sleep(Duration::from_millis(5)); }
            remote.notify().unwrap(); ack.send(()).unwrap();
        }
    });
    for iteration in 0..32 {
        let delay = iteration % 2 != 0;
        send.send(delay).unwrap();
        if !delay { acknowledged.recv().unwrap(); }
        let start = Instant::now();
        assert_eq!(poller.wait(&mut events, Some(Duration::from_secs(1)))?, 0);
        assert!(start.elapsed() < Duration::from_millis(500), "lost notify");
        if delay { acknowledged.recv().unwrap(); }
    }
    drop(send); worker.join().unwrap();
    let other = Poller::new()?;
    poller.notify()?;
    let start = Instant::now();
    assert_eq!(other.wait(&mut events, Some(Duration::from_millis(20)))?, 0);
    assert!(start.elapsed() >= Duration::from_millis(10), "another selector consumed the signal");
    let start = Instant::now(); poller.wait(&mut events, Some(Duration::from_secs(1)))?;
    assert!(start.elapsed() < Duration::from_millis(500));
    for _ in 0..40 {
        let foreign_drop = Poller::new()?;
        std::thread::spawn(move || drop(foreign_drop)).join().unwrap();
        drop(Poller::new()?); // reap retired signal allocation on the owner
    }
    let mut socket = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))?;
    socket.set_nonblocking(true)?; socket.write_all(b"delayed\n")?;
    unsafe { poller.add(&socket, Event::readable(7))?; }
    assert_eq!(poller.wait(&mut events, Some(Duration::from_secs(2)))?, 1);
    assert!(events.iter().any(|e| e.key == 7 && e.readable));
    let foreign = poller.clone();
    let raw = socket.as_raw_fd();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(5));
        let borrowed = unsafe { BorrowedFd::borrow_raw(raw) };
        assert_eq!(foreign.modify(borrowed, Event::readable(7)).unwrap_err().kind(), io::ErrorKind::Unsupported);
        assert_eq!(foreign.delete(borrowed).unwrap_err().kind(), io::ErrorKind::Unsupported);
        assert_eq!(foreign.wait(&mut Events::new(), Some(Duration::ZERO)).unwrap_err().kind(), io::ErrorKind::Unsupported);
        foreign.notify().unwrap();
    });
    events.clear(); poller.wait(&mut events, Some(Duration::from_secs(1)))?;
    worker.join().unwrap();

    events.clear(); assert_eq!(poller.wait(&mut events, Some(Duration::from_millis(10)))?, 0);
    poller.modify_with_mode(&socket, Event::readable(7), PollMode::Level)?;
    assert_eq!(poller.wait(&mut events, Some(Duration::ZERO))?, 1);
    events.clear(); assert_eq!(poller.wait(&mut events, Some(Duration::ZERO))?, 1);
    let mut second = TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))?;
    second.set_nonblocking(true)?; second.write_all(b"fast\n")?;
    unsafe { poller.add_with_mode(&second, Event::readable(8), PollMode::Level)?; }
    // Ensure the second reply is ready without consuming either socket's data.
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        events.clear(); poller.wait(&mut events, Some(Duration::from_millis(100)))?;
        if events.iter().any(|event| event.key == 8) { break; }
        assert!(Instant::now() < deadline, "second fixture reply was not ready");
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut small = Events::with_capacity(NonZeroUsize::new(1).unwrap());
    let mut keys = Vec::new();
    for _ in 0..4 {
        small.clear(); assert_eq!(poller.wait(&mut small, Some(Duration::from_secs(1)))?, 1);
        keys.push(small.iter().next().unwrap().key);
    }
    assert!(keys.contains(&7) && keys.contains(&8), "level source starved: {keys:?}");
    poller.delete(&second)?;
    let mut reply = [0]; assert_eq!(second.read(&mut reply)?, 1); assert_eq!(reply, *b"R");
    poller.delete(&socket)?;
    assert!(unsafe { poller.add_with_mode(&socket, Event::readable(7), PollMode::Edge) }.is_err());
    let mut byte = [0]; assert_eq!(socket.read(&mut byte)?, 1); assert_eq!(byte, *b"R");
    println!("PASS selector signal isolation, 32 notify races, level/oneshot, signal lifecycle");
    Ok(())
}

struct WakeState { ready: bool, waker: Option<Waker> }
struct RemoteWake(Arc<Mutex<WakeState>>);
impl Future for RemoteWake {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.0.lock().unwrap();
        if state.ready { Poll::Ready(()) } else { state.waker = Some(cx.waker().clone()); Poll::Pending }
    }
}
fn wake_gate() {
    for index in 0..32 {
        let state = Arc::new(Mutex::new(WakeState { ready: false, waker: None }));
        let remote = state.clone();
        let worker = std::thread::spawn(move || {
            if index % 2 != 0 { std::thread::sleep(Duration::from_millis(5)); }
            let waker = { let mut state = remote.lock().unwrap(); state.ready = true; state.waker.take() };
            if let Some(waker) = waker { waker.wake(); }
        });
        let woke = async_io::block_on(future::race(async { RemoteWake(state).await; true }, async { async_io::Timer::after(Duration::from_secs(1)).await; false }));
        assert!(woke, "foreign future wake lost"); worker.join().unwrap();
    }
    println!("PASS 32 cross-thread future wakes, all test workers joined");
}

async fn exchange(endpoint: SocketAddr, request: &[u8]) -> io::Result<()> {
    let mut socket = async_net::TcpStream::connect(endpoint).await?;
    let _clone = socket.clone(); // local shared handle, no BSD descriptor duplication
    socket.write_all(request).await?;
    let mut byte = [0]; socket.read_exact(&mut byte).await?; assert_eq!(byte, *b"R");
    assert_eq!(socket.read(&mut byte).await?, 0);
    Ok(())
}
async fn consumer_gate(endpoint: SocketAddr, refused: SocketAddr) -> io::Result<()> {
    let start = Instant::now();
    let (reply, timer) = future::zip(exchange(endpoint, b"delayed\n"), async {
        async_io::Timer::after(Duration::from_millis(20)).await; start.elapsed()
    }).await;
    reply?; assert!(timer >= Duration::from_millis(10) && timer < Duration::from_millis(200));
    assert!(start.elapsed() >= Duration::from_millis(200));
    let (one, two) = future::zip(exchange(endpoint, b"fast\n"), exchange(endpoint, b"delayed\n")).await;
    one?; two?;
    let mut socket = async_net::TcpStream::connect(endpoint).await?;
    socket.write_all(b"cancel\n").await?;
    let cancelled = future::race(async { let _ = socket.read(&mut [0]).await; false }, async {
        async_io::Timer::after(Duration::from_millis(5)).await; true
    }).await;
    assert!(cancelled); drop(socket);
    for _ in 0..80 { exchange(endpoint, b"fast\n").await?; }
    for _ in 0..16 {
        let result = future::race(async_net::TcpStream::connect(refused), async {
            async_io::Timer::after(Duration::from_secs(1)).await;
            Err(io::Error::new(io::ErrorKind::TimedOut, "refused connect stalled"))
        }).await;
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::ConnectionRefused);
    }
    assert_eq!(async_net::TcpStream::connect(("localhost", endpoint.port())).await.unwrap_err().kind(), io::ErrorKind::Unsupported);
    assert_eq!(async_net::TcpStream::connect((std::net::Ipv6Addr::LOCALHOST, endpoint.port())).await.unwrap_err().kind(), io::ErrorKind::Unsupported);
    println!("PASS real async-net concurrent TCP, timer race, cancellation/drop, 80 lifecycles, 16 refusals, EOF");
    Ok(())
}
fn main() -> io::Result<()> {
    let endpoint = std::env::args().nth(1).expect("numeric fixture endpoint").parse().unwrap();
    let refused = std::env::args().nth(2).expect("numeric refusal endpoint").parse().unwrap();
    println!("START async-net2.0.0 async-io2.6.0 polling3.11.0 owner-task v11");
    selector_gate(endpoint)?; wake_gate();
    async_io::block_on(future::race(consumer_gate(endpoint, refused), async {
        async_io::Timer::after(Duration::from_secs(30)).await;
        Err(io::Error::new(io::ErrorKind::TimedOut, "overall consumer deadline"))
    }))?;
    TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))?.write_all(b"done\n")?;
    println!("PASS real async consumer acceptance"); Ok(())
}
