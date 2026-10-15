//! Deterministic unwind, cleanup, payload and thread acceptance.
use std::{
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
struct Guard(Arc<AtomicUsize>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[inline(never)]
fn raise(drops: Arc<AtomicUsize>, value: usize) {
    let _guard = Guard(drops);
    resume_unwind(Box::new(value));
}
fn main() {
    println!("START unwind acceptance");
    let drops = Arc::new(AtomicUsize::new(0));
    let result = catch_unwind(AssertUnwindSafe(|| raise(drops.clone(), 41)));
    assert_eq!(*result.unwrap_err().downcast::<usize>().unwrap(), 41);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    println!("PASS resume_unwind payload and destructor");
    let nested = catch_unwind(AssertUnwindSafe(|| {
        let payload = catch_unwind(AssertUnwindSafe(|| raise(drops.clone(), 42))).unwrap_err();
        resume_unwind(payload);
    }))
    .unwrap_err();
    assert_eq!(*nested.downcast::<usize>().unwrap(), 42);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    println!("PASS catch and rethrow");
    let mut workers = Vec::new();
    for id in 0..4 {
        let drops = drops.clone();
        workers.push(std::thread::spawn(move || {
            for iteration in 0..16 {
                let value = id * 16 + iteration;
                let payload =
                    catch_unwind(AssertUnwindSafe(|| raise(drops.clone(), value))).unwrap_err();
                assert_eq!(*payload.downcast::<usize>().unwrap(), value);
            }
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(drops.load(Ordering::SeqCst), 66);
    println!("PASS 64 caught worker unwinds, destructors and joins");
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let drops2 = drops.clone();
    let worker = std::thread::spawn(move || {
        let _guard = Guard(drops2);
        std::panic::panic_any(99usize);
    });
    let payload = worker.join().unwrap_err();
    assert_eq!(*payload.downcast::<usize>().unwrap(), 99);
    std::panic::set_hook(previous);
    assert_eq!(drops.load(Ordering::SeqCst), 67);
    println!("PASS thread panic payload and destructor");
    println!("DONE unwind acceptance");
}
