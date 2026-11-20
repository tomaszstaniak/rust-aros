use core::mem::{align_of, offset_of, size_of};
use libc::{sigaction, siginfo_t, sigval, SA_RESTART, SA_SIGINFO};

const _: () = {
    assert!(SA_RESTART == 8 && SA_SIGINFO == 16);
    assert!(size_of::<sigval>() == 8 && align_of::<sigval>() == 8);
    assert!(size_of::<sigaction>() == 32 && align_of::<sigaction>() == 8);
    assert!(offset_of!(sigaction, sa_handler) == 0);
    assert!(offset_of!(sigaction, sa_mask) == 8);
    assert!(offset_of!(sigaction, sa_flags) == 24);
    assert!(size_of::<siginfo_t>() == 64 && align_of::<siginfo_t>() == 8);
    assert!(offset_of!(siginfo_t, si_pid) == 16);
    assert!(offset_of!(siginfo_t, si_uid) == 24);
    assert!(offset_of!(siginfo_t, si_addr) == 32);
    assert!(offset_of!(siginfo_t, si_status) == 40);
    assert!(offset_of!(siginfo_t, si_band) == 48);
    assert!(offset_of!(siginfo_t, si_value) == 56);
};


use core::sync::atomic::{AtomicI32, Ordering};

static RECEIVED: AtomicI32 = AtomicI32::new(0);

unsafe extern "C" fn record_signal(
    signum: libc::c_int,
    _info: *mut siginfo_t,
    _context: *mut libc::c_void,
) {
    RECEIVED.store(signum, Ordering::Relaxed);
}

extern "C" {
    fn raise(signum: libc::c_int) -> libc::c_int;
}

fn main() {
    let mut action: sigaction = unsafe { core::mem::zeroed() };
    let mut old: sigaction = action;
    action.sa_flags = SA_RESTART;
    action.set_siginfo_handler(record_signal);
    assert_eq!(action.sa_flags, SA_RESTART | SA_SIGINFO);
    unsafe {
        assert_eq!(libc::sigemptyset(&mut action.sa_mask), 0);
        assert_eq!(libc::sigaction(libc::SIGUSR2, &action, &mut old), 0);
        let delivered = raise(libc::SIGUSR2);
        assert_eq!(libc::sigaction(libc::SIGUSR2, &old, core::ptr::null_mut()), 0);
        assert_eq!(delivered, 0);
    }
    assert_eq!(RECEIVED.load(Ordering::Relaxed), libc::SIGUSR2);
    println!("SIGNAL_ABI_OK");
}
