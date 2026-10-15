use std::thread::JoinHandle;
#[derive(Default)]
pub struct Workers(Vec<JoinHandle<()>>);
impl Workers {
    pub fn spawn(&mut self, f: impl FnOnce() + Send + 'static) -> Result<(), &'static str> {
        self.join(false)?;
        self.0.push(
            std::thread::Builder::new()
                .spawn(f)
                .map_err(|_| "could not spawn request worker")?,
        );
        Ok(())
    }
    pub fn finish(&mut self) -> Result<(), &'static str> {
        self.join(true)
    }
    fn join(&mut self, all: bool) -> Result<(), &'static str> {
        let mut failed = false;
        let mut index = 0;
        while index < self.0.len() {
            if all || self.0[index].is_finished() {
                failed |= self.0.swap_remove(index).join().is_err();
            } else {
                index += 1;
            }
        }
        if failed {
            Err("request worker panicked")
        } else {
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    };
    #[test]
    fn finish_joins_all_workers_before_returning() {
        let finished = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        let mut workers = Workers::default();
        let count = finished.clone();
        workers
            .spawn(move || {
                rx.recv().unwrap();
                count.fetch_add(1, Ordering::SeqCst);
            })
            .unwrap();
        let (done_tx, done_rx) = mpsc::channel();
        let finisher = std::thread::spawn(move || done_tx.send(workers.finish()).unwrap());
        assert!(done_rx
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err());
        tx.send(()).unwrap();
        assert_eq!(
            done_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            Ok(())
        );
        finisher.join().unwrap();
        assert_eq!(finished.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn panic_is_reported_after_other_workers_are_joined() {
        let mut workers = Workers::default();
        workers.spawn(|| panic!("expected worker panic")).unwrap();
        let finished = Arc::new(AtomicUsize::new(0));
        let count = finished.clone();
        // A previously completed panic can be reported during spawn/reap.
        let spawn = workers.spawn(move || {
            count.fetch_add(1, Ordering::SeqCst);
        });
        let finish = workers.finish();
        assert!(spawn.is_err() || finish.is_err());
        if spawn.is_ok() {
            assert_eq!(finished.load(Ordering::SeqCst), 1);
        }
    }
}
