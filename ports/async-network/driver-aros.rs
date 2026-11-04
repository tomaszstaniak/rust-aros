//! Owner-task block_on for AROS. No fallback thread can drive this SocketBase.
use std::{cell::Cell, future::Future, pin::pin, sync::Arc, task::{Context, Poll, Wake, Waker}};
use crate::reactor::Reactor;
unsafe extern "C" { fn rs_selector_owner() -> i32; }
pub(crate) fn init() {}
struct Notify(&'static Reactor);
impl Wake for Notify {
    fn wake(self: Arc<Self>) { self.0.notify(); }
    fn wake_by_ref(self: &Arc<Self>) { self.0.notify(); }
}
thread_local! { static DRIVING: Cell<bool> = const { Cell::new(false) }; }
struct Driving;
impl Drop for Driving { fn drop(&mut self) { DRIVING.with(|flag| flag.set(false)); } }
/// Drive futures on the socket-owning task. Nested calls and foreign-task
/// calls panic explicitly; use concurrent local futures inside one block_on.
pub fn block_on<T>(future: impl Future<Output=T>) -> T {
    assert_eq!(unsafe { rs_selector_owner() }, 0, "AROS block_on requires the socket-owning task");
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
