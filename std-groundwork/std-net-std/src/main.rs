//! std::net through libc-aros: one TCP request, reported to stdout.
//!
//!   std-net-std <host:port>
//!
//! Exit 0: connected and got an HTTP status line. 3: connect failed (the
//! expected result without a TCP/IP stack is ENETDOWN, not a requester).
//! 4: connected but the reply was not HTTP.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

fn main() {
    let target = std::env::args().nth(1).unwrap_or_else(|| "10.0.2.2:8000".into());
    let addr: SocketAddr = target.parse().expect("host:port as an IP address");
    println!("connect {addr}");
    let mut s = match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
        Ok(s) => s,
        Err(e) => {
            println!("connect error: {e} (raw_os_error={:?})", e.raw_os_error());
            std::process::exit(3);
        }
    };
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s.write_all(b"GET /probe.txt HTTP/1.0\r\nHost: probe\r\n\r\n").unwrap();
    let mut reply = Vec::new();
    let r = s.read_to_end(&mut reply);
    let text = String::from_utf8_lossy(&reply);
    println!("read: {:?}, {} bytes", r.map(|_| ()), reply.len());
    println!("status: {}", text.lines().next().unwrap_or(""));
    println!("body-last-line: {}", text.lines().last().unwrap_or(""));
    std::process::exit(if text.starts_with("HTTP/") { 0 } else { 4 });
}
