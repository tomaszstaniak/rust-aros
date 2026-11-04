//! Runtime matrix for an AROS std profile. Each subcommand is one case; the
//! guest script records its exit code with `Echo $RC` right after it. A case
//! prints PASS markers, but only the receipt counts as its result.
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CString};
use std::hash::{BuildHasher, RandomState};
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::{mpsc, Arc, Barrier, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs, thread};

unsafe extern "C" {
    fn getentropy(buf: *mut c_void, len: usize) -> c_int;
}
#[cfg(target_abi = "abiv1")]
unsafe extern "C" {
    fn fputs(s: *const c_char, file: *mut c_void) -> c_int;
    // Mainline <stdio.h> with posixc: `stdout` is __posixc_getstdout().
    #[link_name = "__posixc_getstdout"]
    fn c_stdout() -> *mut c_void;
}

fn env_case() {
    for name in ["RUSTV1_LOCAL", "RUSTV1_GLOBAL"] {
        println!("shell {name}={:?}", env::var(name));
    }
    unsafe { env::set_var("RUSTV1_SET", "value-1") };
    assert_eq!(env::var("RUSTV1_SET").as_deref(), Ok("value-1"));
    assert!(env::vars().any(|(k, v)| k == "RUSTV1_SET" && v == "value-1"), "vars() lacks RUSTV1_SET");
    unsafe { env::set_var("RUSTV1_SET", "value-2") };
    assert_eq!(env::var("RUSTV1_SET").as_deref(), Ok("value-2"));
    unsafe { env::remove_var("RUSTV1_SET") };
    assert!(env::var("RUSTV1_SET").is_err());
    println!("args={:?} current_dir={:?} current_exe={:?}", env::args().collect::<Vec<_>>(),
             env::current_dir(), env::current_exe());
    println!("PASS env set/get/vars/remove");
}

fn files_case(base: &str) {
    let root = format!("{base}rustv1-files");
    let _ = fs::remove_dir_all(&root);
    let deep = format!("{root}/a/b/c");
    fs::create_dir_all(&deep).unwrap();
    for n in 0..20 {
        fs::write(format!("{deep}/f{n}.txt"), format!("file {n}\n")).unwrap();
    }
    let names: Vec<_> = fs::read_dir(&deep).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(names.len(), 20, "read_dir count");
    let path = format!("{root}/data.bin");
    let mut f = fs::OpenOptions::new().create(true).write(true).read(true).truncate(true).open(&path).unwrap();
    f.write_all(&[7u8; 10000]).unwrap();
    f.seek(SeekFrom::Start(5000)).unwrap();
    f.write_all(b"MID").unwrap();
    f.seek(SeekFrom::Start(4999)).unwrap();
    let mut buf = [0u8; 5];
    f.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, &[7, b'M', b'I', b'D', 7]);
    f.set_len(6000).unwrap();
    f.sync_all().unwrap();
    drop(f);
    let meta = fs::metadata(&path).unwrap();
    assert!(meta.is_file() && meta.len() == 6000, "len {}", meta.len());
    println!("modified={:?}", meta.modified().map(|t| t.duration_since(UNIX_EPOCH)));
    let mut a = fs::OpenOptions::new().append(true).open(&path).unwrap();
    a.write_all(b"tail").unwrap();
    drop(a);
    assert_eq!(fs::metadata(&path).unwrap().len(), 6004);
    fs::rename(&path, format!("{root}/renamed.bin")).unwrap();
    assert!(fs::metadata(&path).is_err());
    fs::copy(format!("{root}/renamed.bin"), format!("{root}/copy.bin")).unwrap();
    assert_eq!(fs::read(format!("{root}/copy.bin")).unwrap().len(), 6004);
    assert!(fs::metadata(&root).unwrap().is_dir());
    println!("canonicalize={:?}", fs::canonicalize(&deep));
    fs::remove_file(format!("{root}/copy.bin")).unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert!(fs::metadata(&root).is_err(), "remove_dir_all left {root}");
    println!("PASS files {base} dirs=3 files=20 seek/set_len/append/rename/copy/remove_dir_all");
}

fn time_case() {
    let start = Instant::now();
    thread::sleep(Duration::from_millis(250));
    let slept = start.elapsed();
    assert!(slept >= Duration::from_millis(200), "slept {slept:?}");
    let mut last = Instant::now();
    for _ in 0..1000 {
        let now = Instant::now();
        assert!(now >= last, "Instant went backwards");
        last = now;
    }
    let wall = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    // 2026-01-01 .. 2038-01-19: AROS time_t is 32 bits.
    assert!((1_767_225_600..2_147_483_647).contains(&wall), "wall {wall}");
    println!("PASS time slept={slept:?} unix={wall}");
}

fn random_case() {
    let (a, b) = (RandomState::new(), RandomState::new());
    let mut same = 0;
    for k in 0..16u64 {
        if a.hash_one(k) == b.hash_one(k) { same += 1; }
    }
    assert!(same < 16, "two RandomStates hash identically");
    let map: HashMap<String, usize> = (0..1000).map(|n| (n.to_string(), n)).collect();
    assert_eq!(map["999"], 999);
    let mut x = [0u8; 32];
    let mut y = [0u8; 32];
    assert_eq!(unsafe { getentropy(x.as_mut_ptr().cast(), 32) }, 0, "getentropy");
    assert_eq!(unsafe { getentropy(y.as_mut_ptr().cast(), 32) }, 0, "getentropy");
    assert!(x != y && x != [0; 32], "entropy repeated or zero");
    println!("PASS random hashmap=1000 randomstate-differ getentropy=2x32 (quality not assessed)");
}

fn threads_case() {
    let counter = Arc::new(Mutex::new(0u64));
    let (tx, rx) = mpsc::sync_channel(2);
    let handles: Vec<_> = (0..8).map(|id| {
        let (counter, tx) = (counter.clone(), tx.clone());
        thread::Builder::new().name(format!("w{id}")).spawn(move || {
            for _ in 0..1000 { *counter.lock().unwrap() += 1; }
            tx.send((id, thread::current().name().unwrap().to_owned())).unwrap();
        }).unwrap()
    }).collect();
    drop(tx);
    let got: Vec<_> = rx.iter().collect();
    assert_eq!(got.len(), 8);
    assert!(got.iter().all(|(id, name)| *name == format!("w{id}")));
    for h in handles { h.join().unwrap(); }
    assert_eq!(*counter.lock().unwrap(), 8000);
    let pair = (Mutex::new(false), Condvar::new());
    let start = Instant::now();
    let (guard, timeout) = pair.1.wait_timeout(pair.0.lock().unwrap(), Duration::from_millis(300)).unwrap();
    assert!(timeout.timed_out() && !*guard);
    drop(guard);
    let waited = start.elapsed();
    assert!(waited >= Duration::from_millis(200) && waited < Duration::from_secs(5), "condvar {waited:?}");
    let shared = Arc::new((Mutex::new(false), Condvar::new()));
    let other = shared.clone();
    let notifier = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        *other.0.lock().unwrap() = true;
        other.1.notify_all();
    });
    let mut ready = shared.0.lock().unwrap();
    while !*ready { ready = shared.1.wait(ready).unwrap(); }
    drop(ready);
    notifier.join().unwrap();
    let lock = Arc::new(RwLock::new(5));
    let barrier = Arc::new(Barrier::new(4));
    let readers: Vec<_> = (0..4).map(|_| {
        let (lock, barrier) = (lock.clone(), barrier.clone());
        thread::spawn(move || { barrier.wait(); *lock.read().unwrap() })
    }).collect();
    assert!(readers.into_iter().all(|r| r.join().unwrap() == 5));
    *lock.write().unwrap() = 6;
    let mut data = vec![0u32; 64];
    thread::scope(|s| {
        for (i, chunk) in data.chunks_mut(16).enumerate() {
            s.spawn(move || chunk.iter_mut().for_each(|v| *v = i as u32));
        }
    });
    assert_eq!(data.iter().sum::<u32>(), 16 * (0 + 1 + 2 + 3));
    println!("PASS threads counter=8000 sync_channel=8 names condvar-timeout={waited:?} notify rwlock barrier scope parallelism={:?}",
             thread::available_parallelism());
}

static FINISHED: AtomicUsize = AtomicUsize::new(0);
fn detach_case() {
    for _ in 0..64 {
        let (tx, rx) = mpsc::channel();
        let h = thread::spawn(move || { FINISHED.fetch_add(1, SeqCst); tx.send(()).unwrap(); });
        rx.recv().unwrap();
        thread::sleep(Duration::from_millis(20));
        drop(h); // detach after the thread has finished
    }
    for _ in 0..16 {
        drop(thread::spawn(|| { thread::sleep(Duration::from_millis(100)); FINISHED.fetch_add(1, SeqCst); }));
    }
    let start = Instant::now();
    while FINISHED.load(SeqCst) != 80 {
        assert!(start.elapsed() < Duration::from_secs(10), "detached workers did not finish");
        thread::sleep(Duration::from_millis(10));
    }
    println!("PASS detach after-finish=64 before-finish=16; exit receipt follows");
}

fn rayon_case() {
    use rayon::prelude::*;
    // An explicit global pool: on a one-CPU guest the default pool has a
    // single worker, which would not exercise exit with several workers.
    rayon::ThreadPoolBuilder::new().num_threads(4).build_global().unwrap();
    let sum: u64 = (1u64..=100_000).into_par_iter().sum();
    assert_eq!(sum, 5_000_050_000);
    let mut v: Vec<u32> = (0..50_000).rev().collect();
    v.par_sort();
    assert!(v.windows(2).all(|w| w[0] <= w[1]));
    println!("PASS rayon global threads={} sum par_sort; returning without pool shutdown", rayon::current_num_threads());
}

fn stdout_case() {
    for n in 0..200 { println!("rust line {n:03}"); }
    #[cfg(target_abi = "abiv1")]
    unsafe {
        let out = c_stdout();
        for n in 0..50 {
            let line = CString::new(format!("c fputs line {n:03}\n")).unwrap();
            fputs(line.as_ptr(), out);
        }
    }
    print!("rust tail without newline");
    // No explicit flush: std's exit path and the C library's must deliver.
}

fn recurse(depth: u32, acc: &mut u64) -> u64 {
    let frame = [depth as u8; 1024];
    *acc = acc.wrapping_add(std::hint::black_box(&frame)[(depth % 1024) as usize] as u64);
    if depth == 0 { *acc } else { recurse(depth - 1, acc) + 1 }
}

fn bigstack_case() {
    let h = thread::Builder::new().stack_size(16 << 20).spawn(|| {
        let mut acc = 0;
        recurse(8000, &mut acc) // about 8 MiB of 1 KiB frames
    }).unwrap();
    let depth = h.join().unwrap();
    assert!(depth >= 8000);
    println!("PASS bigstack thread stack=16MiB recursion=8000x1KiB");
}

fn would_block<T: std::fmt::Debug>(r: Result<T, fs::TryLockError>) -> bool {
    matches!(r, Err(fs::TryLockError::WouldBlock))
}

fn open_rw(path: &str) -> fs::File {
    fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path).unwrap()
}

/// File locks within one program: two descriptors, another thread, drop.
fn flock_case(path: &str) {
    let a = open_rw(path);
    let b = open_rw(path);
    a.try_lock().expect("first try_lock");
    assert!(would_block(b.try_lock()), "second descriptor got the exclusive lock");
    assert!(would_block(b.try_lock_shared()), "second descriptor got a shared lock");
    a.unlock().unwrap();
    b.try_lock().expect("try_lock after unlock");
    b.unlock().unwrap();
    a.lock_shared().unwrap();
    b.lock_shared().unwrap();
    assert!(would_block(open_rw(path).try_lock()), "exclusive beside two shared");
    a.unlock().unwrap();
    b.unlock().unwrap();
    b.unlock().unwrap(); // unlocking an unlocked descriptor is a no-op
    a.lock().unwrap();
    let path2 = path.to_owned();
    let other = thread::spawn(move || {
        let c = open_rw(&path2);
        let busy = would_block(c.try_lock());
        (busy, c)
    });
    let (busy, c) = other.join().unwrap();
    assert!(busy, "another thread got the exclusive lock");
    drop(a); // closing the locking descriptor releases its lock
    let path3 = path.to_owned();
    let freed = thread::spawn(move || {
        let d = open_rw(&path3);
        let ok = d.try_lock().is_ok();
        d.unlock().unwrap();
        ok
    }).join().unwrap();
    assert!(freed, "drop did not release the lock");
    drop((b, c));
    fs::remove_file(path).unwrap();
    println!("PASS flock same-process: second fd WouldBlock (ex and sh), unlock, shared x2, other thread WouldBlock, drop releases");
}

/// Holds a lock for `secs` seconds and ends without unlocking.
fn flock_hold(path: &str, secs: u64, shared: bool) {
    let f = open_rw(path);
    if shared { f.lock_shared().unwrap() } else { f.lock().unwrap() }
    fs::write(format!("{path}.held"), b"held\n").unwrap();
    println!("holding {} lock on {path} for {secs}s", if shared { "shared" } else { "exclusive" });
    thread::sleep(Duration::from_secs(secs));
    std::mem::forget(f); // exit with the lock and the descriptor still open
    println!("PASS flock-hold exiting with the lock held");
}

fn flock_other(path: &str, expect: &str) {
    let f = open_rw(path);
    let ex = f.try_lock();
    let ex_blocked = would_block(ex);
    if !ex_blocked { f.unlock().unwrap(); }
    let sh = f.try_lock_shared();
    let sh_blocked = would_block(sh);
    if !sh_blocked { f.unlock().unwrap(); }
    println!("other process: exclusive {} shared {}", if ex_blocked { "WouldBlock" } else { "Ok" },
             if sh_blocked { "WouldBlock" } else { "Ok" });
    match expect {
        "busy" => assert!(ex_blocked && sh_blocked, "lock of another process not seen"),
        "shared" => assert!(ex_blocked && !sh_blocked, "shared lock of another process wrong"),
        "free" => assert!(!ex_blocked && !sh_blocked, "lock not released at the holder's exit"),
        _ => panic!("expect busy|shared|free"),
    }
    println!("PASS flock-other expect={expect}");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let case = args.get(1).map(String::as_str).unwrap_or("hello");
    match case {
        "hello" => println!("PASS hello from Rust std"),
        "env" => env_case(),
        "files" => files_case(args.get(2).map(String::as_str).unwrap_or("RAM:")),
        "time" => time_case(),
        "random" => random_case(),
        "threads" => threads_case(),
        "detach" => detach_case(),
        "rayon" => rayon_case(),
        "rayon-default" => {
            use rayon::prelude::*;
            let sum: u64 = (1u64..=10_000).into_par_iter().sum();
            assert_eq!(sum, 50_005_000);
            println!("PASS rayon default threads={} parallelism={:?}", rayon::current_num_threads(), thread::available_parallelism());
        }
        "stdout" => stdout_case(),
        "bigstack" => bigstack_case(),
        "panic" => panic!("intentional panic in main (panic=abort)"),
        "panic-thread" => {
            let _ = thread::spawn(|| panic!("intentional panic in a worker (panic=abort)")).join();
            println!("UNEXPECTED: worker panic did not abort");
        }
        "flock" => flock_case(args.get(2).map(String::as_str).unwrap_or("RAM:rust-flock.txt")),
        "flock-hold" => flock_hold(&args[2], args[3].parse().unwrap(), args.get(4).map(String::as_str) == Some("sh")),
        "flock-other" => flock_other(&args[2], &args[3]),
        "exit3" => return ExitCode::from(3),
        other => { eprintln!("unknown case {other}"); return ExitCode::from(64); }
    }
    ExitCode::SUCCESS
}
