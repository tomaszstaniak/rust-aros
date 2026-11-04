from pathlib import Path
import json
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class SelectorDelivery(unittest.TestCase):
    def test_level_ready_sources_make_progress_with_capacity_one(self):
        # Compile the real Rust backend; replace only its AROS syscall boundary.
        backend = json.dumps(str(ROOT / 'ports/async-network/polling-aros.rs'))
        harness = r'''
extern crate self as libc;
#[allow(non_camel_case_types)]
#[repr(C)] pub struct pollfd { pub fd: i32, pub events: i16, pub revents: i16 }
#[derive(Debug, Copy, Clone)]
pub struct Event { key: usize, readable: bool, writable: bool, extra: backend::EventExtra }
#[derive(Debug, Copy, Clone)] pub enum PollMode { Oneshot, Level, Edge }
#[path = BACKEND] mod backend;
use std::{ffi::c_void, time::Instant};
#[no_mangle] extern "C" fn rs_selector_new() -> *mut c_void { 1usize as *mut c_void }
#[no_mangle] extern "C" fn rs_selector_check(_: *mut c_void) -> i32 { 0 }
#[no_mangle] extern "C" fn rs_selector_free(_: *mut c_void) {}
#[no_mangle] extern "C" fn rs_selector_notify(_: *mut c_void) -> i32 { 0 }
#[no_mangle] extern "C" fn rs_selector_validate(_: *mut c_void, _: i32) -> i32 { 0 }
#[no_mangle] unsafe extern "C" fn rs_selector_wait(_: *mut c_void, fds: *mut pollfd, n: usize, _: i64) -> i32 {
    for fd in std::slice::from_raw_parts_mut(fds, n) { fd.revents = fd.events; }
    n as i32
}
fn main() {
    let poller = backend::Poller::new().unwrap();
    for fd in [1, 2] {
        let event = Event { key: fd as usize, readable: true, writable: false, extra: backend::EventExtra::empty() };
        poller.add(fd, event, PollMode::Level).unwrap();
    }
    let mut events = backend::Events::with_capacity(1);
    let mut delivered = Vec::new();
    for _ in 0..4 {
        events.clear();
        poller.wait_deadline(&mut events, Some(Instant::now())).unwrap();
        assert_eq!(events.iter().count(), 1);
        delivered.push(events.iter().next().unwrap().key);
    }
    assert_eq!(delivered, [1, 2, 1, 2], "level source starved behind capacity limit");
}
'''.replace('BACKEND', backend)
        with tempfile.TemporaryDirectory(prefix='aros-selector-') as directory:
            source = Path(directory) / 'test.rs'; source.write_text(harness)
            binary = Path(directory) / 'check'
            result = subprocess.run(['rustc', '+nightly', '--edition=2021', str(source), '-o', str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = subprocess.run([str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
