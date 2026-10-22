#![no_std]

extern "C" {
    fn aros_probe_write(text: *const core::ffi::c_char);
    fn abort() -> !;
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    unsafe { abort() }
}

#[no_mangle]
pub extern "C" fn aros_acceptance(_: *const core::ffi::c_char) -> i32 {
    // Volatile operands keep this a code-generation check on the target CPU.
    let input = 12345678901234567890u128;
    let n = unsafe { core::ptr::read_volatile(&input) };
    if n / 97 != 127275040218913071 || n % 97 != 3 {
        return 20;
    }
    unsafe {
        aros_probe_write(b"no_std: work complete; awaiting CRT exit receipt\n\0".as_ptr().cast());
    }
    0
}
