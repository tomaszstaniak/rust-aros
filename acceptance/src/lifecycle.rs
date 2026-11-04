//! Detect a retained CURRENT reference through the Thread-owned name allocation.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering::SeqCst};

const NAME_LEN: usize = 4093;
static LIVE_NAMES: AtomicIsize = AtomicIsize::new(0);
static DROPS: AtomicUsize = AtomicUsize::new(0);

pub struct TrackingAllocator;
fn tracked(layout: Layout) -> bool {
    layout.align() == 1 && matches!(layout.size(), NAME_LEN | 4094)
}
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() && tracked(layout) { LIVE_NAMES.fetch_add(1, SeqCst); }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if tracked(layout) { LIVE_NAMES.fetch_sub(1, SeqCst); }
        unsafe { System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, size) };
        if !result.is_null() {
            if tracked(layout) { LIVE_NAMES.fetch_sub(1, SeqCst); }
            if tracked(Layout::from_size_align(size, layout.align()).unwrap()) {
                LIVE_NAMES.fetch_add(1, SeqCst);
            }
        }
        result
    }
}

struct Late(Cell<bool>);
impl Drop for Late {
    fn drop(&mut self) {
        if self.0.get() {
            assert_eq!(std::thread::current().name().unwrap().len(), NAME_LEN);
            DROPS.fetch_add(1, SeqCst);
        }
    }
}
struct First(Cell<bool>);
impl Drop for First {
    fn drop(&mut self) {
        if self.0.get() {
            assert_eq!(std::thread::current().name().unwrap().len(), NAME_LEN);
            // Initialize another value during destruction. Cleanup must wait
            // for its destructor too, regardless of pthread key ordering.
            LATE.with(|v| v.0.set(true));
            DROPS.fetch_add(1, SeqCst);
        }
    }
}
thread_local! {
    static FIRST: First = const { First(Cell::new(false)) };
    static LATE: Late = const { Late(Cell::new(false)) };
}

pub fn check() {
    // Create the pthread keys before entering pthread's destructor callback:
    // AROS holds its TLS-key semaphore while invoking those callbacks.
    FIRST.with(|_| {});
    LATE.with(|_| {});
    let baseline = LIVE_NAMES.load(SeqCst);
    let drops = DROPS.load(SeqCst);
    for _ in 0..16 {
        let worker = std::thread::Builder::new().name("n".repeat(NAME_LEN)).spawn(|| {
            FIRST.with(|v| v.0.set(true));
        }).unwrap();
        let handle = worker.thread().clone();
        worker.join().unwrap();
        assert_eq!(LIVE_NAMES.load(SeqCst), baseline + 1, "retained handle owns name");
        drop(handle);
        assert_eq!(LIVE_NAMES.load(SeqCst), baseline, "CURRENT leaked its owning reference");
    }
    assert_eq!(DROPS.load(SeqCst) - drops, 32, "TLS destructors must run exactly once");
    println!("thread lifecycle: names balanced=16 TLS drops=32");
}
