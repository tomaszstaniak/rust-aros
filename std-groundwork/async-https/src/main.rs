mod tls;
use futures_lite::future;
use std::{net::SocketAddr, time::Duration};
use tls::{Error, Tls};
async fn exchange(endpoint: SocketAddr, ca: &[u8], name: &str) -> Result<(), Error> {
    exchange_checked(endpoint, ca, name, false).await
}
async fn exchange_checked(
    endpoint: SocketAddr,
    ca: &[u8],
    name: &str,
    check_owner: bool,
) -> Result<(), Error> {
    let mut tls = Tls::connect(endpoint, ca, name).await?;
    if check_owner {
        for _ in 0..4 {
            let foreign_ca = ca.to_vec();
            let error = std::thread::spawn(move || {
                futures_lite::future::block_on(async {
                    match Tls::connect(endpoint, &foreign_ca, "rust-aros.test").await {
                        Ok(_) => panic!("foreign TLS constructor accepted"),
                        Err(e) => e,
                    }
                })
            })
            .join()
            .unwrap();
            assert_eq!(error.domain, 4, "{error:?}");
        }
        println!("PASS foreign TLS construction rejected; owner context preserved");
    }
    tls.write_all(b"GET /gate HTTP/1.1\r\nHost: rust-aros.test\r\nConnection: close\r\n\r\n")
        .await?;
    let mut response = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        let n = tls.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buffer[..n]);
        assert!(response.len() <= 8192, "unbounded response");
    }
    assert_eq!(
        response,
        b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nverified"
    );
    tls.close().await?;
    Ok(())
}
async fn gate(host: &str, ca: &[u8], other_ca: &[u8]) -> Result<(), Error> {
    let valid: SocketAddr = format!("{host}:18771").parse().unwrap();
    exchange_checked(valid, ca, "rust-aros.test", true).await?;
    println!("PASS verified HTTPS with authenticated EOF");
    for (port, name, trust, flag) in [
        (18771, "wrong.test", ca, 4),
        (18772, "rust-aros.test", ca, 1),
        (18771, "rust-aros.test", other_ca, 8),
    ] {
        let err = match Tls::connect(format!("{host}:{port}").parse().unwrap(), trust, name).await {
            Ok(_) => panic!("certificate accepted"),
            Err(e) => e,
        };
        assert_eq!(err.domain, 2, "{err:?}");
        assert_ne!(err.verify & flag, 0, "{err:?}");
        println!("PASS certificate rejection port={port} flag={flag} {err:?}");
    }
    // Complete entropy setup and TCP connect before timing pending TLS work.
    // Poll handshake first: randomized race can expire during synchronous setup.
    let mut pending = Tls::begin(
        format!("{host}:18775").parse().unwrap(),
        ca,
        "rust-aros.test",
    )
    .await?;
    let cancelled = future::or(
        async {
            pending
                .handshake()
                .await
                .expect("pending fixture handshake failed before cancellation");
            false
        },
        async {
            async_io::Timer::after(Duration::from_secs(2)).await;
            true
        },
    )
    .await;
    drop(pending);
    assert!(cancelled, "stalled handshake was not cancelled");
    let live = Tls::connect(
        format!("{host}:18776").parse().unwrap(),
        ca,
        "rust-aros.test",
    )
    .await?;
    drop(live); // no implicit close_notify or blocking network operation
    exchange(valid, ca, "rust-aros.test").await?;
    println!("PASS pending handshake cancellation, live drop, and subsequent connection");
    let (a, b) = future::zip(
        exchange(valid, ca, "rust-aros.test"),
        exchange(valid, ca, "rust-aros.test"),
    )
    .await;
    a?;
    b?;
    println!("PASS cleanup after rejection and concurrent TLS context lifetime");
    let err = exchange(
        format!("{host}:18774").parse().unwrap(),
        ca,
        "rust-aros.test",
    )
    .await
    .unwrap_err();
    assert_eq!(err.domain, 5, "truncated TLS must fail: {err:?}");
    println!("PASS raw transport EOF rejected");
    Ok(())
}
fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    let ca = std::fs::read(&args[2])?;
    let other_ca = std::fs::read(&args[3])?;
    println!(
        "START TLS Mbed3.6.7 jitterentropy3.7 x86_64-v11 run={}",
        env!("AROS_TLS_RUN_ID")
    );
    if args.get(4).map(String::as_str) == Some("entropy-fail") {
        async_io::block_on(async {
            let err = match Tls::connect(
                format!("{}:18773", args[1]).parse().unwrap(),
                &ca,
                "rust-aros.test",
            )
            .await
            {
                Ok(_) => panic!("entropy unexpectedly accepted"),
                Err(e) => e,
            };
            assert_eq!(err.domain, 3, "{err:?}");
            assert_eq!(err.code, -148, "expected PSA insufficient entropy: {err:?}");
            assert!(err.message.contains("jitterentropy=-2"), "{err:?}");
            println!("PASS forced entropy failure before connect {err:?}");
        });
    } else {
        async_io::block_on(future::race(gate(&args[1], &ca, &other_ca), async {
            async_io::Timer::after(Duration::from_secs(40)).await;
            Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "HTTPS gate deadline",
            )))
        }))?;
    }
    println!("PASS HTTPS acceptance run={}", env!("AROS_TLS_RUN_ID"));
    Ok(())
}
