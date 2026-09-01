//! exec.library memory, with the pairing rule enforced by the compiler.
//!
//! `AllocMem` must be matched by a `FreeMem` *with the same size* — a classic
//! source of Amiga heap corruption. [`Mem`] remembers the size and frees on
//! drop, so it cannot be got wrong.
use crate::sys;
use core::ops::{Deref, DerefMut};

pub struct Mem {
    ptr: *mut u8,
    size: usize,
}

impl Mem {
    /// Allocate `size` bytes. `MEMF_CLEAR` is applied, so the block is zeroed.
    pub fn new(size: usize) -> Option<Mem> {
        let ptr = unsafe { sys::AllocMem(size as u64, sys::MEMF_CLEAR) };
        if ptr.is_null() {
            None
        } else {
            Some(Mem { ptr, size })
        }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

impl Deref for Mem {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.ptr, self.size) }
    }
}

impl DerefMut for Mem {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

impl Drop for Mem {
    fn drop(&mut self) {
        unsafe { sys::FreeMem(self.ptr, self.size as u64) }
    }
}
