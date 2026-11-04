//! Minimal runtime probes for std threads on native AROS (aarch64 Pi first).
//! Each case prints one `CASE <name> <PASS|FAIL> <detail>` line; the run ends with `DONE`.
//! Usage: atp <case> [args]; cases: cwd, open <path>, alloc <secs>, xfree <rounds>, errno <path>, all <file-dos> <file-nix>
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

fn out(s: &str) {
    let mut o = std::io::stdout();
    let _ = writeln!(o, "{s}");
    let _ = o.flush();
}

fn read_file(p: &str) -> Result<usize, String> {
    let mut f = std::fs::File::open(p).map_err(|e| format!("open: {e} kind={:?} raw={:?}", e.kind(), e.raw_os_error()))?;
    let mut v = Vec::new();
    f.read_to_end(&mut v).map_err(|e| format!("read: {e}"))?;
    Ok(v.len())
}

fn on_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    thread::Builder::new().name("probe-worker".into()).stack_size(256 * 1024).spawn(f).unwrap().join().unwrap()
}

fn case_cwd() {
    let m = std::env::current_dir().map(|p| p.display().to_string()).map_err(|e| e.to_string());
    let t = on_thread(|| std::env::current_dir().map(|p| p.display().to_string()).map_err(|e| e.to_string()));
    out(&format!("CASE cwd {} main={m:?} thread={t:?}", if m == t { "PASS" } else { "FAIL" }));
}

fn case_open(p: &str) {
    let m = read_file(p);
    let pp = p.to_string();
    let t = on_thread(move || read_file(&pp));
    let ok = m.is_ok() && m == t;
    out(&format!("CASE open[{p}] {} main={m:?} thread={t:?}", if ok { "PASS" } else { "FAIL" }));
}

fn case_errno(p: &str) {
    let probe = |p: &str| match std::fs::File::open(p) {
        Ok(_) => "opened?!".to_string(),
        Err(e) => format!("kind={:?} raw={:?}", e.kind(), e.raw_os_error()),
    };
    let m = probe(p);
    let pp = p.to_string();
    let t = on_thread(move || {
        let a = probe(&pp);
        // a second failure after a success-free gap, to see a stale cell
        let b = probe(&pp);
        format!("{a} / {b}")
    });
    let ok = m.contains("NotFound") && t.matches("NotFound").count() == 2;
    out(&format!("CASE errno[{p}] {} main={m} thread={t}", if ok { "PASS" } else { "FAIL" }));
}

// xorshift, no rand crate
fn rng(s: &mut u64) -> u64 { *s ^= *s << 13; *s ^= *s >> 7; *s ^= *s << 17; *s }

fn churn(seed: u64, stop: Arc<AtomicBool>, ops: Arc<AtomicU64>) -> Result<(), String> {
    let mut s = seed | 1;
    let mut live: Vec<Vec<u8>> = Vec::new();
    let mut strs: Vec<String> = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        for _ in 0..64 {
            let r = rng(&mut s);
            let n = (r % 4096) as usize + 1;
            let tag = (r >> 32) as u8;
            if live.len() < 256 && r & 1 == 0 {
                live.push(vec![tag; n]);
            } else if !live.is_empty() {
                let i = (r as usize >> 8) % live.len();
                let v = live.swap_remove(i);
                if v.iter().any(|&b| b != v[0]) { return Err("vec content changed".into()); }
            }
            if r & 6 == 0 {
                let mut st = String::with_capacity(8);
                for k in 0..(n % 200) { st.push((b'a' + (k % 26) as u8) as char); }
                strs.push(st);
                if strs.len() > 128 { strs.drain(..64); }
            }
            ops.fetch_add(1, Ordering::Relaxed);
        }
    }
    Ok(())
}

fn case_alloc(secs: u64) {
    let stop = Arc::new(AtomicBool::new(false));
    let ops = Arc::new(AtomicU64::new(0));
    let hs: Vec<_> = (0..4u64)
        .map(|i| { let (s, o) = (stop.clone(), ops.clone()); thread::Builder::new().stack_size(256 * 1024).spawn(move || churn(0x9e3779b97f4a7c15 ^ i, s, o)).unwrap() })
        .collect();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(secs) { thread::sleep(Duration::from_millis(200)); }
    stop.store(true, Ordering::Relaxed);
    let res: Vec<_> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    let ok = res.iter().all(|r| r.is_ok());
    out(&format!("CASE alloc4x{secs}s {} ops={} {:?}", if ok { "PASS" } else { "FAIL" }, ops.load(Ordering::Relaxed), res));
}

fn case_xfree(rounds: usize) {
    // thread allocates, main frees
    let (tx, rx) = mpsc::channel::<Box<[u8]>>();
    let h = thread::spawn(move || {
        for i in 0..rounds { let _ = tx.send(vec![(i & 0xff) as u8; 32 + (i * 37) % 3000].into_boxed_slice()); }
    });
    let mut n = 0usize;
    for b in rx { if b.iter().any(|&x| x != b[0]) { out("CASE xfree FAIL content"); return; } drop(b); n += 1; }
    h.join().unwrap();
    // main allocates, thread frees
    let (tx2, rx2) = mpsc::channel::<String>();
    let h2 = thread::spawn(move || { let mut c = 0usize; for s in rx2 { c += s.len(); drop(s); } c });
    for i in 0..rounds { tx2.send("x".repeat(16 + (i * 53) % 2000)).unwrap(); }
    drop(tx2);
    let c = h2.join().unwrap();
    out(&format!("CASE xfree{rounds} PASS t->m={n} m->t bytes={c}"));
}

fn case_write(path: &str) {
    let pp = path.to_string();
    let t = on_thread(move || -> Result<usize, String> {
        let mut f = std::fs::File::create(&pp).map_err(|e| format!("create: {e}"))?;
        f.write_all(b"written from a thread\n").map_err(|e| e.to_string())?;
        drop(f);
        read_file(&pp)
    });
    out(&format!("CASE write-thread[{path}] {} {t:?}", if t == Ok(22) { "PASS" } else { "FAIL" }));
}

// Four threads each open/read/stat/close their own file (by the path given + index) for `secs`.
// A shared path-conversion buffer in the C library shows here as errors or wrong bytes.
fn fs_worker(base: String, i: usize, stop: Arc<AtomicBool>) -> (u64, u64, String) {
    let path = format!("{base}{i}.txt");
    let want = format!("file {i} {}\n", "y".repeat(40 + i));
    if let Err(e) = std::fs::write(&path, &want) { return (0, 1, format!("write {path}: {e}")); }
    let (mut ok, mut bad, mut first) = (0u64, 0u64, String::new());
    while !stop.load(Ordering::Relaxed) {
        let r = std::fs::metadata(&path).map_err(|e| format!("stat: {e}")).and_then(|m| {
            if m.len() as usize != want.len() { return Err(format!("len {}", m.len())); }
            std::fs::read_to_string(&path).map_err(|e| format!("read: {e}"))
        });
        match r {
            Ok(s) if s == want => ok += 1,
            Ok(s) => { bad += 1; if first.is_empty() { first = format!("wrong bytes {:?}", &s[..s.len().min(20)]); } }
            Err(e) => { bad += 1; if first.is_empty() { first = e; } }
        }
    }
    let _ = std::fs::remove_file(&path);
    (ok, bad, first)
}

fn case_fsrace(secs: u64, base: &str, threads: usize) {
    let stop = Arc::new(AtomicBool::new(false));
    let hs: Vec<_> = (0..threads)
        .map(|i| { let (s, b) = (stop.clone(), base.to_string()); thread::Builder::new().stack_size(256 * 1024).spawn(move || fs_worker(b, i, s)).unwrap() })
        .collect();
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(secs) { thread::sleep(Duration::from_millis(200)); }
    stop.store(true, Ordering::Relaxed);
    let res: Vec<_> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    let ok = res.iter().all(|r| r.1 == 0 && r.0 > 0);
    out(&format!("CASE fsrace{threads}x{secs}s[{base}] {} {res:?}", if ok { "PASS" } else { "FAIL" }));
}

// Threads whose JoinHandle is dropped: while they still run (detached running) and after they
// finished (detached finished), as std::thread::spawn without join does. Each reads a file.
fn case_detach(n: usize, path: &str) {
    let done = Arc::new(AtomicU64::new(0));
    for i in 0..n {
        let (d, p) = (done.clone(), path.to_string());
        let h = thread::Builder::new().stack_size(256 * 1024).spawn(move || {
            let v = std::fs::read(&p).map(|b| b.len()).unwrap_or(0);
            let s: Vec<String> = (0..50).map(|k| format!("{k}-{v}")).collect();
            d.fetch_add(s.len() as u64 / 50, Ordering::SeqCst);
        });
        match h {
            Ok(h) => {
                if i % 2 == 1 {
                    // let it finish first, then detach
                    while !h.is_finished() { thread::sleep(Duration::from_millis(5)); }
                }
                drop(h);
            }
            Err(e) => { out(&format!("CASE detach FAIL spawn {i}: {e}")); return; }
        }
        if i % 8 == 7 { thread::sleep(Duration::from_millis(20)); }
    }
    let t0 = Instant::now();
    while done.load(Ordering::SeqCst) < n as u64 && t0.elapsed() < Duration::from_secs(30) { thread::sleep(Duration::from_millis(50)); }
    let d = done.load(Ordering::SeqCst);
    thread::sleep(Duration::from_secs(2));
    out(&format!("CASE detach{n} {} finished={d}", if d == n as u64 { "PASS" } else { "FAIL" }));
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |i: usize| a.get(i).cloned().unwrap_or_default();
    match arg(1).as_str() {
        "cwd" => case_cwd(),
        "open" => case_open(&arg(2)),
        "errno" => case_errno(&arg(2)),
        "alloc" => case_alloc(arg(2).parse().unwrap_or(20)),
        "xfree" => case_xfree(arg(2).parse().unwrap_or(2000)),
        "write" => case_write(&arg(2)),
        "mixed" => {
            // allocation churn on four threads while two threads do path calls
            let secs: u64 = arg(2).parse().unwrap_or(20);
            let h = thread::spawn(move || case_alloc(secs));
            case_fsrace(secs, &arg(3), 2);
            h.join().unwrap();
        }
        "detach" => case_detach(arg(2).parse().unwrap_or(200), &arg(3)),
        "fsrace" => case_fsrace(arg(2).parse().unwrap_or(20), &arg(3), arg(4).parse().unwrap_or(4)),
        other => out(&format!("unknown case {other:?}")),
    }
    out("DONE");
}
