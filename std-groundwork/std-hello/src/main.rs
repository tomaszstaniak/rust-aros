//! A plain Rust program. Every facility below is std, with no unsafe and
//! nothing AROS-specific in the source.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn check(name: &str, ok: bool) { println!("  [{}] {}", if ok { "ok" } else { "FAIL" }, name); }

fn main() {
    println!("std on AROS x86_64");

    // time
    let t0 = Instant::now();
    std::thread::sleep(Duration::from_millis(150));
    let el = t0.elapsed();
    check("thread::sleep + Instant", el >= Duration::from_millis(100) && el < Duration::from_secs(2));
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    check("SystemTime is sane", now > 1_600_000_000);

    // fs
    let path = "RAM:std-test.txt";
    std::fs::write(path, "written by std::fs\n").unwrap();
    let back = std::fs::read_to_string(path).unwrap();
    check("fs::write / read_to_string", back == "written by std::fs\n");
    let md = std::fs::metadata(path).unwrap();
    check("fs::metadata len", md.len() == 19 && md.is_file());
    let names: Vec<String> = std::fs::read_dir("RAM:").unwrap()
        .filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    check("fs::read_dir RAM:", names.iter().any(|n| n == "std-test.txt"));
    std::fs::remove_file(path).unwrap();
    check("fs::remove_file", std::fs::metadata(path).is_err());

    // threads + sync
    let counter = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let handles: Vec<_> = (0..4).map(|i| {
        let c = counter.clone();
        std::thread::spawn(move || { for _ in 0..1000 { *c.lock().unwrap() += 1; } i })
    }).collect();
    let ids: u32 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    check("4 threads, Arc<Mutex>", *counter.lock().unwrap() == 4000 && ids == 6);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || tx.send("hello over a channel").unwrap());
    check("mpsc channel", rx.recv().unwrap() == "hello over a channel");

    // collections (HashMap needs the random seed from getentropy)
    let mut hm = HashMap::new();
    for w in "the quick brown fox jumps over the lazy dog the end".split(' ') { *hm.entry(w).or_insert(0) += 1; }
    check("HashMap", hm["the"] == 3 && hm.len() == 9);

    // process
    let out = std::process::Command::new("echo").arg("from a child").output();
    match out {
        Ok(o) => check("process::Command output", String::from_utf8_lossy(&o.stdout).contains("from a child")),
        Err(e) => println!("  [FAIL] process::Command: {e}"),
    }

    // net
    match std::net::TcpStream::connect_timeout(&"10.0.2.2:8080".parse().unwrap(), Duration::from_secs(3)) {
        Ok(mut s) => {
            s.write_all(b"GET /std HTTP/1.0\r\n\r\n").unwrap();
            let mut buf = String::new(); s.read_to_string(&mut buf).unwrap();
            check("net::TcpStream HTTP", buf.starts_with("HTTP/1.0 200"));
        }
        Err(e) => println!("  [FAIL] net::TcpStream: {e}"),
    }

    println!("done");
}
