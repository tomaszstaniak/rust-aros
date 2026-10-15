from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
HARNESS = r'''
extern crate self as getrandom;
extern crate self as libc;
use std::sync::Mutex;
static READS: Mutex<(Vec<usize>, usize)> = Mutex::new((Vec::new(), 0));
#[derive(Debug)]
pub struct Error;
impl Error { pub fn new_custom(_: u16) -> Self { Self } }
pub unsafe fn getentropy(dest: *mut std::ffi::c_void, len: usize) -> i32 {
    let mut state = READS.lock().unwrap();
    state.0.push(len);
    unsafe { std::ptr::write_bytes(dest.cast::<u8>(), 0x5a, len); }
    if state.0.len() == state.1 { -1 } else { 0 }
}
#[test]
fn chunks_requests_and_erases_prior_chunks_after_failure() {
    let mut bytes = [0xcc; 513];
    unsafe { __getrandom_v03_custom(bytes.as_mut_ptr(), bytes.len()).unwrap(); }
    assert_eq!(READS.lock().unwrap().0, vec![256,256,1]);
    assert!(bytes.iter().all(|&b| b == 0x5a));
    *READS.lock().unwrap() = (Vec::new(), 2);
    assert!(unsafe { __getrandom_v03_custom(bytes.as_mut_ptr(), bytes.len()) }.is_err());
    assert_eq!(READS.lock().unwrap().0, vec![256,256]);
    assert!(bytes.iter().all(|&b| b == 0));
    unsafe { __getrandom_v03_custom(std::ptr::null_mut(), 0).unwrap(); }
    assert_eq!(READS.lock().unwrap().0.len(), 2);
    *READS.lock().unwrap() = (Vec::new(), 0);
    let mut uninit = std::mem::MaybeUninit::<[u8; 513]>::uninit();
    unsafe { __getrandom_v03_custom(uninit.as_mut_ptr().cast(), 513).unwrap(); }
    let initialized = unsafe { uninit.assume_init() };
    assert!(initialized.iter().all(|&b| b == 0x5a));
    assert_eq!(READS.lock().unwrap().0, vec![256,256,1]);
}
'''


class GetrandomBackend(unittest.TestCase):
    def test_chunking_and_mid_request_failure(self):
        source = ROOT / 'std-groundwork/entropy-std/src/aros_random.rs'
        # Compile the production callback against an injected libc source;
        # stripping only module documentation allows inclusion at crate root.
        implementation = '\n'.join(line for line in source.read_text().splitlines()
                                   if not line.startswith('//!'))
        with tempfile.TemporaryDirectory(prefix='aros-getrandom-') as directory:
            root = Path(directory)
            harness = root / 'harness.rs'
            harness.write_text(HARNESS + '\n' + implementation)
            binary = root / 'check'
            result = subprocess.run(['rustc', '--edition=2021', '--test', str(harness),
                                     '-o', str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            result = subprocess.run([str(binary)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
