//! std::process without fork: run a command, capture its output, get its
//! exit code — on AROS that is dos.library's SystemTagList with SYS_Output
//! pointed at a PIPE: file. Every call here is a plain linkable symbol.
#![no_std]
#![no_main]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use aros::{aros_main, println};
use core::ffi::c_void;

type Bptr = usize;
#[repr(C)] struct TagItem { tag: usize, data: usize }

extern "C" {
    fn SystemTagList(cmd: *const u8, tags: *const TagItem) -> i32;
    fn Open(name: *const u8, mode: i32) -> Bptr;
    fn Close(f: Bptr) -> i32;
    fn Read(f: Bptr, buf: *mut c_void, len: i32) -> i32;
    fn IoErr() -> i32;
}

const TAG_USER: usize = 1 << 31;
const TAG_DONE: usize = 0;
const SYS_INPUT: usize = TAG_USER + 32 + 1;
const SYS_OUTPUT: usize = TAG_USER + 32 + 2;
const SYS_ASYNCH: usize = TAG_USER + 32 + 3;
const MODE_OLDFILE: i32 = 1005;
const MODE_NEWFILE: i32 = 1006;

/// The shape std::process::Output has: status + captured stdout.
struct Output { status: i32, stdout: Vec<u8> }

/// Run `cmd` in a child process with stdout captured. Synchronous: SystemTagList
/// returns the command's exit code once it finishes. The pipe is opened for
/// reading first, on a name unique to this call, so the child can never block
/// on a writer with no reader.
fn spawn_capture(cmd: &str, tag: u32) -> Result<Output, i32> {
    let mut pipe_out = String::from("PIPE:oxc_");
    let mut n = tag; let mut digits = Vec::new();
    loop { digits.push(b'0' + (n % 10) as u8); n /= 10; if n == 0 { break; } }
    while let Some(d) = digits.pop() { pipe_out.push(d as char); }
    pipe_out.push('\0');

    let mut c = String::from(cmd); c.push('\0');
    unsafe {
        let wr = Open(pipe_out.as_ptr(), MODE_NEWFILE);
        if wr == 0 { return Err(IoErr()); }
        let nil = Open(b"NIL:\0".as_ptr(), MODE_OLDFILE);
        let tags = [
            TagItem { tag: SYS_INPUT,  data: nil },
            TagItem { tag: SYS_OUTPUT, data: wr },
            TagItem { tag: SYS_ASYNCH, data: 1 },   // run in the background, we read meanwhile
            TagItem { tag: TAG_DONE,   data: 0 },
        ];
        // With SYS_Asynch the handles are closed by the child; the pipe's
        // writer end therefore closes exactly when the command finishes, and
        // our reader sees EOF.
        let rc = SystemTagList(c.as_ptr(), tags.as_ptr());
        if rc == -1 { return Err(IoErr()); }

        let rd = Open(pipe_out.as_ptr(), MODE_OLDFILE);
        if rd == 0 { return Err(IoErr()); }
        let mut out = Vec::new();
        let mut buf = [0u8; 512];
        loop {
            let n = Read(rd, buf.as_mut_ptr() as *mut c_void, buf.len() as i32);
            if n <= 0 { break; }
            out.extend_from_slice(&buf[..n as usize]);
        }
        Close(rd);
        Ok(Output { status: rc, stdout: out })
    }
}

fn show(label: &str, r: Result<Output, i32>) {
    match r {
        Ok(o) => {
            let text = core::str::from_utf8(&o.stdout).unwrap_or("<non-utf8>");
            println!("  [ok] {:<28} status={} stdout={} bytes", label, o.status, o.stdout.len());
            for line in text.lines().take(4) { println!("       | {}", line); }
        }
        Err(e) => println!("  [FAIL] {:<26} IoErr={}", label, e),
    }
}

/// Synchronous variant: SystemTagList blocks until the child exits and returns
/// its return code, which is what std's `status()` wants. Output goes to NIL:.
fn spawn_status(cmd: &str) -> i32 {
    let mut c = String::from(cmd); c.push('\0');
    unsafe {
        let nil = Open(b"NIL:\0".as_ptr(), MODE_NEWFILE);
        let tags = [TagItem { tag: SYS_OUTPUT, data: nil }, TagItem { tag: TAG_DONE, data: 0 }];
        let rc = SystemTagList(c.as_ptr(), tags.as_ptr());
        Close(nil);
        rc
    }
}

fn run() {
    println!("process-test: spawning commands through SystemTagList");
    println!("  sync status: echo={} failat-style 'quit 10'={} missing={}",
             spawn_status("echo x"), spawn_status("quit 10"), spawn_status("this-does-not-exist"));
    show("echo hello", spawn_capture("echo \"hello from a child process\"", 1));
    show("version", spawn_capture("version", 2));
    show("list RAM:", spawn_capture("list RAM: LFORMAT=\"%n\"", 3));
    show("nonexistent command", spawn_capture("this-does-not-exist", 4));
    println!("process-test: done");
}
aros_main!(run);
