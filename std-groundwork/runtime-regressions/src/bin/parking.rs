use parking_lot::{Condvar, Mutex};
use std::{sync::Arc, thread, time::{Duration, Instant}};
unsafe extern "C" { fn deadline_bad() -> u32; fn deadline_calls() -> u32; }
fn main() {
    println!("RUN {} parking begin", env!("RUNTIME_RUN_ID"));
    let mutex = Mutex::new(false);
    let condvar = Condvar::new();
    let mut guard = mutex.lock();
    let begin = Instant::now();
    assert!(condvar.wait_for(&mut guard, Duration::from_millis(300)).timed_out());
    let elapsed = begin.elapsed();
    assert!(!*guard);
    assert!(elapsed >= Duration::from_millis(150) && elapsed < Duration::from_secs(3), "timeout elapsed={elapsed:?}");
    drop(guard);
    println!("PASS timeout elapsed_us={}", elapsed.as_micros());
    for _ in 0..8 {
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let mut guard = pair.0.lock();
        let other = pair.clone();
        let join = thread::spawn(move || {
            // Acquiring the same mutex proves wait_for has released it.
            let mut state = other.0.lock();
            *state = true;
            other.1.notify_one();
        });
        while !*guard {
            assert!(!pair.1.wait_for(&mut guard, Duration::from_secs(2)).timed_out(), "wakeup timed out");
        }
        drop(guard);
        join.join().unwrap();
    }
    unsafe {
        // Native ARM has no unix target family and uses parking_lot's generic
        // parker. Require the declared backend's actual contract, rather than
        // treating absence of a pthread timedwait as a clock failure.
        if option_env!("RUNTIME_PARKER") == Some("generic") {
            assert_eq!(deadline_calls(), 0);
        } else {
            assert!(deadline_calls() >= 1);
        }
        assert_eq!(deadline_bad(), 0);
    }
    println!("PASS parker-backend={}", option_env!("RUNTIME_PARKER").unwrap_or("unix"));
    println!("RUN {} parking pass wakeups=8", env!("RUNTIME_RUN_ID"));
}
