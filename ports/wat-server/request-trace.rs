//! Optional bounded lifecycle trace; emitted only after the server loop ends.
use lspt::NumberOrString;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex, OnceLock,
};
static DROPPED: AtomicUsize = AtomicUsize::new(0);
static ENABLED: OnceLock<bool> = OnceLock::new();
static EVENTS: OnceLock<Mutex<Vec<(&'static str, NumberOrString)>>> = OnceLock::new();
pub fn record(event: &'static str, id: &NumberOrString) {
    if !*ENABLED.get_or_init(|| std::env::args().any(|arg| arg == "--trace-requests")) {
        return;
    }
    let mut events = EVENTS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap();
    if events.len() < 512 {
        events.push((event, id.clone()));
    } else {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}
pub fn dump() {
    if DROPPED.load(Ordering::Relaxed) > 0 {
        eprintln!("REQUEST_TRACE_DROPPED {}", DROPPED.load(Ordering::Relaxed));
    }
    if let Some(events) = EVENTS.get() {
        for (sequence, (event, id)) in events.lock().unwrap().iter().enumerate() {
            eprintln!("REQUEST_TRACE {sequence} {event} {id:?}");
        }
    }
}
