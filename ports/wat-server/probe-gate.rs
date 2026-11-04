use std::sync::{Condvar, Mutex};
use std::time::Duration;
#[derive(Default)]
pub struct Gate {
    released: Mutex<bool>,
    wake: Condvar,
}
impl Gate {
    pub fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.wake.notify_all();
    }
    pub fn wait(&self, deadline: Duration) -> bool {
        let (released, _) = self
            .wake
            .wait_timeout_while(self.released.lock().unwrap(), deadline, |released| {
                !*released
            })
            .unwrap();
        *released
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc};
    #[test]
    fn release_before_wait_is_retained() {
        let gate = Gate::default();
        gate.release();
        assert!(gate.wait(Duration::from_millis(50)));
    }
    #[test]
    fn worker_cannot_pass_before_release() {
        let gate = Arc::new(Gate::default());
        let worker_gate = gate.clone();
        let (tx, rx) = mpsc::channel();
        let worker =
            std::thread::spawn(move || tx.send(worker_gate.wait(Duration::from_secs(2))).unwrap());
        assert!(rx.recv_timeout(Duration::from_millis(30)).is_err());
        gate.release();
        assert!(rx.recv_timeout(Duration::from_secs(1)).unwrap());
        worker.join().unwrap();
    }
    #[test]
    fn absent_release_times_out_as_failure() {
        assert!(!Gate::default().wait(Duration::from_millis(10)));
    }
}
