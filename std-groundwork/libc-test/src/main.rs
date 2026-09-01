//! Does the generated libc actually match AROS at runtime?
//!
//! Compiling proves nothing about struct layouts: a wrong offset gives a
//! wrong file size, garbage filenames or a thread that never runs. Each check
//! below compares against a value we know independently.
#![no_std]
#![no_main]

extern crate alloc;

use aros::{aros_main, println};
use core::ffi::c_void;
use core::ptr;

const PATH: &[u8] = b"RAM:libc-test.txt\0";
const BODY: &[u8] = b"AROS libc bindings check\n";

fn check(name: &str, ok: bool, detail: &str) {
    println!("  [{}] {:<26} {}", if ok { "ok" } else { "FAIL" }, name, detail);
}

extern "C" fn worker(arg: *mut c_void) -> *mut c_void {
    unsafe { *(arg as *mut u64) = 0xC0FFEE };
    ptr::null_mut()
}

fn run() {
    println!("libc-test: checking the generated bindings against the real system");

    // --- time ---------------------------------------------------------
    unsafe {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        let r = libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
        // Anything after 2020 and before the 2038 rollover of a 32-bit time_t.
        let sane = r == 0 && ts.tv_sec > 1_577_836_800 && ts.tv_nsec < 1_000_000_000;
        check("clock_gettime REALTIME", sane, "");
        println!("        tv_sec={} tv_nsec={}", ts.tv_sec, ts.tv_nsec);

        let mut a = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        let mut b = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut a);
        let nap = libc::timespec { tv_sec: 0, tv_nsec: 200_000_000 };
        libc::nanosleep(&nap, ptr::null_mut());
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut b);
        let d = (b.tv_sec - a.tv_sec) as i64 * 1_000_000_000 + (b.tv_nsec - a.tv_nsec);
        check("nanosleep 200ms", d > 100_000_000 && d < 2_000_000_000, "");
        println!("        monotonic delta = {} ns", d);
    }

    // --- write, stat, read back ---------------------------------------
    unsafe {
        let fd = libc::open(PATH.as_ptr() as *const _,
                            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_int);
        check("open O_WRONLY|O_CREAT", fd >= 0, "");
        if fd >= 0 {
            let n = libc::write(fd, BODY.as_ptr() as *const c_void, BODY.len() - 1);
            check("write", n == (BODY.len() - 1) as isize, "");
            libc::close(fd);
        }

        let mut st: libc::stat = core::mem::zeroed();
        let r = libc::stat(PATH.as_ptr() as *const _, &mut st);
        let want = (BODY.len() - 1) as i64;
        // If the layout were wrong, st_size would not equal what we wrote.
        check("stat st_size", r == 0 && st.st_size == want, "");
        println!("        st_size={} (wrote {}), st_mode=0o{:o}", st.st_size, want, st.st_mode);
        check("S_IFMT says regular file",
              (st.st_mode & libc::S_IFMT) == libc::S_IFREG, "");

        let fd = libc::open(PATH.as_ptr() as *const _, libc::O_RDONLY, 0);
        let mut buf = [0u8; 64];
        let n = libc::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
        libc::close(fd);
        check("read back matches", n == want as isize && &buf[..n as usize] == &BODY[..n as usize], "");
        libc::unlink(PATH.as_ptr() as *const _);
    }

    // --- directory listing (exercises the dirent layout) ---------------
    unsafe {
        let dir = libc::opendir(b"RAM:\0".as_ptr() as *const _);
        check("opendir RAM:", !dir.is_null(), "");
        if !dir.is_null() {
            let mut count = 0;
            let mut printable = true;
            loop {
                let e = libc::readdir(dir);
                if e.is_null() { break; }
                let name = &(*e).d_name;
                // A wrong d_name offset shows up immediately as junk.
                let mut len = 0;
                while len < name.len() && name[len] != 0 { len += 1; }
                if len == 0 || len > 120 { printable = false; }
                for i in 0..len {
                    let c = name[i] as u8;
                    if c < 32 || c > 126 { printable = false; }
                }
                if count < 4 {
                    let s = core::str::from_utf8(
                        core::slice::from_raw_parts(name.as_ptr() as *const u8, len)).unwrap_or("?");
                    println!("        entry: {:?} (d_type={})", s, (*e).d_type);
                }
                count += 1;
                if count > 200 { break; }
            }
            libc::closedir(dir);
            check("readdir names are sane", printable && count > 0, "");
        }
    }

    // --- threads -------------------------------------------------------
    unsafe {
        let mut slot: u64 = 0;
        let mut tid: libc::pthread_t = 0;
        let r = libc::pthread_create(&mut tid, ptr::null(), worker, &mut slot as *mut u64 as *mut c_void);
        check("pthread_create", r == 0, "");
        if r == 0 {
            libc::pthread_join(tid, ptr::null_mut());
            check("thread ran and wrote", slot == 0xC0FFEE, "");
        }

        let mut m = libc::pthread_mutex_t::zeroed();
        let ok = libc::pthread_mutex_init(&mut m, ptr::null()) == 0
            && libc::pthread_mutex_lock(&mut m) == 0
            && libc::pthread_mutex_unlock(&mut m) == 0
            && libc::pthread_mutex_destroy(&mut m) == 0;
        check("mutex lifecycle", ok, "");
    }

    println!("libc-test: done");
}

aros_main!(run);
