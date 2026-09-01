//! Can Rust use bsdsocket.library? Opens a TCP connection to the host and
//! sends an HTTP request. QEMU's user network puts the host at 10.0.2.2.
#![no_std]
#![no_main]
extern crate alloc;

use aros::{aros_main, println};
use core::ffi::c_void;

#[repr(C)]
struct SockaddrIn { family: u8, len_or_family: u8, port: u16, addr: u32, zero: [u8; 8] }

extern "C" {
    fn aros_socket_available() -> i32;
    fn aros_socket(d: i32, t: i32, p: i32) -> i32;
    fn aros_connect(s: i32, a: *const SockaddrIn, l: i32) -> i32;
    fn aros_send(s: i32, b: *const c_void, n: isize, f: i32) -> isize;
    fn aros_recv(s: i32, b: *mut c_void, n: isize, f: i32) -> isize;
    fn aros_closesocket(s: i32) -> i32;
    fn aros_sock_errno() -> i32;
}

const AF_INET: i32 = 2;
const SOCK_STREAM: i32 = 1;

fn run() {
    println!("net-test: bsdsocket.library from Rust");
    unsafe {
        if aros_socket_available() == 0 {
            println!("  SocketBase is NULL - no TCP/IP stack running");
            return;
        }
        println!("  [ok] SocketBase opened by libnet at startup");

        let s = aros_socket(AF_INET, SOCK_STREAM, 0);
        println!("  [{}] socket() = {}", if s >= 0 { "ok" } else { "FAIL" }, s);
        if s < 0 { println!("      errno {}", aros_sock_errno()); return; }

        // BSD sockaddr_in: sin_len, sin_family, port and address big-endian.
        let addr = SockaddrIn { family: 16, len_or_family: AF_INET as u8,
                                port: 8080u16.to_be(), addr: u32::from_be_bytes([10, 0, 2, 2]).to_be(),
                                zero: [0; 8] };
        let r = aros_connect(s, &addr, 16);
        println!("  [{}] connect(10.0.2.2:8080) = {} errno {}", if r == 0 { "ok" } else { "FAIL" }, r, aros_sock_errno());
        if r == 0 {
            let req = b"GET /hello-from-aros HTTP/1.0\r\n\r\n";
            let n = aros_send(s, req.as_ptr() as *const c_void, req.len() as isize, 0);
            println!("  [{}] send() = {}", if n > 0 { "ok" } else { "FAIL" }, n);
            let mut buf = [0u8; 200];
            let n = aros_recv(s, buf.as_mut_ptr() as *mut c_void, buf.len() as isize, 0);
            if n > 0 {
                let mut end = 0; while end < n as usize && buf[end] != b'\r' && buf[end] != b'\n' { end += 1; }
                println!("  [ok] recv() = {} bytes, first line: {}", n, core::str::from_utf8(&buf[..end]).unwrap_or("?"));
            } else {
                println!("  [FAIL] recv() = {} errno {}", n, aros_sock_errno());
            }
        }
        aros_closesocket(s);
    }
    println!("net-test: done");
}
aros_main!(run);
