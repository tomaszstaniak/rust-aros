//! Identical first-std acceptance workload for both AROS CPU profiles.
mod lifecycle;
#[global_allocator]
static ALLOCATOR: lifecycle::TrackingAllocator = lifecycle::TrackingAllocator;

use std::collections::{BTreeMap, HashMap};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

#[repr(align(64))]
#[derive(Clone)]
struct Aligned([u8; 64]);

pub fn probe(path: &str) {
    lifecycle::check();
    let mut aligned = Vec::with_capacity(1);
    for n in 0..257 {
        aligned.push(Aligned([n as u8; 64]));
    }
    assert_eq!(aligned.as_ptr() as usize % 64, 0);
    assert_eq!(aligned[255].0, [255; 64]);
    let ordered: BTreeMap<_, _> = (0..128).map(|n| (n, n * n)).collect();
    assert_eq!(ordered[&17], 289);
    let hashed: HashMap<_, _> = (0..128).map(|n| (n.to_string(), n)).collect();
    assert_eq!(hashed["127"], 127);
    println!("collections: ok (including aligned Vec growth)");

    std::fs::write(path, b"AROS Rust std round trip\n").unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"AROS Rust std round trip\n");
    assert_eq!(std::fs::metadata(path).unwrap().len(), 25);
    std::fs::remove_file(path).unwrap();
    println!("file: ok");

    let before = Instant::now();
    let wall = SystemTime::now();
    std::thread::sleep(Duration::from_millis(60));
    let elapsed = before.elapsed();
    assert!(elapsed >= Duration::from_millis(20), "monotonic clock did not advance");
    println!("time: ok elapsed={elapsed:?} wall={wall:?}");

    let counter = Arc::new(Mutex::new(0));
    let (tx, rx) = mpsc::channel();
    let mut workers = Vec::new();
    for id in 0..4 {
        let counter = Arc::clone(&counter);
        let tx = tx.clone();
        workers.push(std::thread::spawn(move || {
            for _ in 0..1000 {
                *counter.lock().unwrap() += 1;
            }
            tx.send(id).unwrap();
            id * id
        }));
    }
    drop(tx);
    let mut received: Vec<_> = rx.iter().collect();
    received.sort();
    assert_eq!(received, [0, 1, 2, 3]);
    let joined: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(joined, [0, 1, 4, 9]);
    assert_eq!(*counter.lock().unwrap(), 4000);
    println!("threads: ok counter=4000 channel=4 joined=4");
}

/// The C harness owns CRT startup and the final exit status.
///
/// # Safety
/// `path` must point to a valid, NUL-terminated UTF-8 private scratch filename.
#[no_mangle]
pub unsafe extern "C" fn aros_acceptance(path: *const std::ffi::c_char) -> i32 {
    let path = unsafe { std::ffi::CStr::from_ptr(path) }.to_str().unwrap();
    probe(path);
    println!("std: work complete; awaiting CRT exit receipt");
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn acceptance_workload_checks_collections_files_time_and_joined_workers() {
        let file = std::env::temp_dir().join(format!("aros-acceptance-{}", std::process::id()));
        super::probe(file.to_str().unwrap());
        assert!(!file.exists());
    }
}
