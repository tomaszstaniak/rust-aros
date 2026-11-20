use std::io::ErrorKind;
use std::sync::atomic::{AtomicUsize, Ordering};

static PREVIOUS: AtomicUsize = AtomicUsize::new(0);
static CALLBACK: AtomicUsize = AtomicUsize::new(0);
unsafe extern "C" fn previous(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    PREVIOUS.fetch_add(1, Ordering::Relaxed);
}

extern "C" {
    fn raise(signal: libc::c_int) -> libc::c_int;
}

fn main() {
    let signal = libc::SIGUSR2;
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        let mut original: libc::sigaction = std::mem::zeroed();
        action.set_siginfo_handler(previous);
        assert_eq!(libc::sigemptyset(&mut action.sa_mask), 0);
        assert_eq!(libc::sigaction(signal, &action, &mut original), 0);

        let rejected = signal_hook_registry::register_sigaction(signal, |_| {}).unwrap_err();
        assert_eq!(rejected.kind(), ErrorKind::Unsupported);
        let id = signal_hook_registry::register(signal, || {
            CALLBACK.fetch_add(1, Ordering::Relaxed);
        }).unwrap();
        assert_eq!(raise(signal), 0);
        assert!(signal_hook_registry::unregister(id));
        assert_eq!(raise(signal), 0);

        assert_eq!(libc::sigaction(signal, &original, std::ptr::null_mut()), 0);
    }
    assert_eq!(CALLBACK.load(Ordering::Relaxed), 1);
    assert_eq!(PREVIOUS.load(Ordering::Relaxed), 2);
    println!("SIGNAL_REGISTRY_OK");
}
