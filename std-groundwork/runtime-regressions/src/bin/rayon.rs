use rayon::prelude::*;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
fn progress(stage: &str) {
    // A hung CRT exit may leave redirected stdout open. Closed, uniquely named
    // checkpoints distinguish completed work from a real Shell exit receipt.
    if let Some(prefix) = option_env!("RUNTIME_PROGRESS_PREFIX") {
        std::fs::write(format!("{prefix}-{stage}.txt"), b"observed; not an exit receipt\n").unwrap();
    }
}
fn main() {
    println!("RUN {} rayon begin shutdown={}", env!("RUNTIME_RUN_ID"), cfg!(feature="shutdown"));
    // Private pool drop is a separate control; all worker exits must complete.
    let exited = Arc::new(AtomicUsize::new(0));
    let counter = exited.clone();
    {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(2)
            .exit_handler(move |_| { counter.fetch_add(1, Ordering::SeqCst); }).build().unwrap();
        assert_eq!(pool.install(|| (1u64..=1000).into_par_iter().sum::<u64>()), 500500);
    }
    let start = std::time::Instant::now();
    while exited.load(Ordering::SeqCst) != 2 {
        assert!(start.elapsed() < std::time::Duration::from_secs(3), "private workers did not exit");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    println!("PASS private-pool worker-exits=2");
    progress("private-workers");
    let global_exits = Arc::new(AtomicUsize::new(0));
    let counter = global_exits.clone();
    #[cfg(feature="shutdown")]
    rayon_core::terminate_global_thread_pool(); // no registry yet: must remain a no-op
    rayon::ThreadPoolBuilder::new().num_threads(2)
        .exit_handler(move |_| { counter.fetch_add(1, Ordering::SeqCst); }).build_global().unwrap();
    assert_eq!((1u64..=1000).into_par_iter().sum::<u64>(), 500500);
    progress("global-work");
    #[cfg(feature="shutdown")] {
        rayon_core::terminate_global_thread_pool();
        rayon_core::terminate_global_thread_pool(); // release exactly once
        let start = std::time::Instant::now();
        while global_exits.load(Ordering::SeqCst) != 2 {
            assert!(start.elapsed() < std::time::Duration::from_secs(3), "global workers did not exit");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        println!("PASS global-pool worker-exits=2");
    }
    progress("before-return");
    println!("RUN {} rayon before-return", env!("RUNTIME_RUN_ID"));
    // Only a subsequent receipt executed by the Shell proves process exit.
}
