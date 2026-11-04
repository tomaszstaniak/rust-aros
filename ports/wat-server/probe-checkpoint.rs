//! Diagnostic only: pause one request at a real Salsa cancellation checkpoint.
use std::{cell::RefCell, sync::OnceLock, time::Duration};
#[path = "probe_gate.rs"]
mod gate;
static GATE: OnceLock<gate::Gate> = OnceLock::new();
thread_local! {
    static NOTIFY: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}
pub fn arm(notify: impl FnOnce() + 'static) {
    NOTIFY.with(|slot| *slot.borrow_mut() = Some(Box::new(notify)));
}
pub fn release() {
    GATE.get_or_init(Default::default).release();
}
pub fn service() -> crate::LanguageService {
    crate::LanguageService {
        storage: salsa::Storage::builder()
            .event_callback(Box::new(|event| {
                if matches!(event.kind, salsa::EventKind::WillCheckCancellation) {
                    let notify = NOTIFY.with(|slot| slot.borrow_mut().take());
                    if let Some(notify) = notify {
                        notify();
                        // A missing notification must fail visibly rather than fabricate cancellation.
                        if !GATE
                            .get_or_init(Default::default)
                            .wait(Duration::from_secs(20))
                        {
                            panic!("AROS_PROBE_GATE_TIMEOUT");
                        }
                    }
                }
            }))
            .build(),
        ..Default::default()
    }
}
