//! getrandom 0.3's explicit custom-backend contract for the v11 example.
//! Select with --cfg getrandom_backend="custom"; export exactly once per app.

/// # Safety
/// Called only by getrandom with a valid writable buffer of `len` bytes.
#[no_mangle]
unsafe fn __getrandom_v03_custom(dest: *mut u8, len: usize) -> Result<(), getrandom::Error> {
    if len == 0 { return Ok(()); }
    // getrandom permits uninitialized output. Do not create a byte slice
    // until the source has initialized it; raw-pointer chunks need no reads.
    let mut offset = 0;
    while offset < len {
        let chunk_len = (len - offset).min(256);
        let chunk = unsafe { dest.add(offset) };
        if unsafe { libc::getentropy(chunk.cast(), chunk_len) } != 0 {
            unsafe { core::ptr::write_bytes(dest, 0, len); }
            return Err(getrandom::Error::new_custom(1));
        }
        offset += chunk_len;
    }
    Ok(())
}
