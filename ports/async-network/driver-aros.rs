//! Owner-task block_on for AROS. No fallback thread can drive this SocketBase.
use std::{cell::Cell, future::Future, pin::pin, sync::Arc, task::{Context, Poll, Wake, Waker}};
use crate::reactor::Reactor;
use std::{io, time::Duration};
unsafe extern "C" { fn rs_reactor_owner() -> i32; }
pub(crate) fn init() {}
struct Notify(&'static Reactor);
impl Wake for Notify {
    fn wake(self: Arc<Self>) { self.0.notify(); }
    fn wake_by_ref(self: &Arc<Self>) { self.0.notify(); }
}
thread_local! { static DRIVING: Cell<bool> = const { Cell::new(false) }; }
struct Driving;
impl Drop for Driving { fn drop(&mut self) { DRIVING.with(|flag| flag.set(false)); } }
/// Initialize the reactor on the startup task before spawning workers that
/// may register timers. Panics if the underlying selector cannot be allocated.
pub fn init_owner() -> io::Result<()> {
    if unsafe { rs_reactor_owner() } != 0 {
        return Err(io::Error::last_os_error());
    }
    Reactor::get();
    Ok(())
}

/// Process ready I/O and timers once from an external event loop.
///
/// Call this on the startup task, which also owns SocketBase when available. The call does not wait for
/// readiness or create a reactor before the first registration. The event
/// loop must call it again to service future readiness. Calling during
/// another reactor drive reports [`io::ErrorKind::WouldBlock`].
pub fn poll_owner() -> io::Result<()> {
    if unsafe { rs_reactor_owner() } != 0 {
        return Err(io::Error::last_os_error());
    }
    let Some(reactor) = Reactor::initialized() else {
        return Ok(());
    };
    if DRIVING.with(|flag| flag.replace(true)) {
        return Err(io::ErrorKind::WouldBlock.into());
    }
    let _guard = Driving;
    reactor.lock().react(Some(Duration::ZERO))
}

/// Drive futures on the socket-owning task. Nested calls and foreign-task
/// calls panic explicitly; use concurrent local futures inside one block_on.
pub fn block_on<T>(future: impl Future<Output=T>) -> T {
    assert_eq!(unsafe { rs_reactor_owner() }, 0, "AROS block_on requires the socket-owning task");
    DRIVING.with(|flag| assert!(!flag.replace(true), "nested AROS block_on is unsupported"));
    let _guard = Driving;
    let reactor = Reactor::get();
    let waker = Waker::from(Arc::new(Notify(reactor)));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(result) = future.as_mut().poll(&mut context) { return result; }
        // notify is an Exec signal that remains pending across this boundary.
        // A wake during poll or immediately before WaitSelect cannot be lost.
        reactor.lock().react(None).expect("AROS reactor wait failed");
    }
}
