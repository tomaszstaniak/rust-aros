#![no_std]
#![no_main]
extern crate alloc;
use aros::{args, aros_main};
use aros_limpet::{Output, QueryStatus};
use core::fmt::Write;

fn run() {
    // This single-task example keeps its process-owned output open.
    let mut out = unsafe { Output::current() };
    let capabilities = out.capabilities();
    let report = alloc::format!(
        "NO_STD query={:?} geometry={:?} rewrite={} ioerr={}\n",
        out.status(),
        capabilities.geometry(),
        capabilities.can_rewrite_line(),
        out.io_error()
    );
    if let Some(path) = args::get(1) {
        aros::fs::write_new(path, report.as_bytes()).unwrap();
    }
    out.set_title("Rust no_std / Limpet").unwrap();
    out.set_ansi(2, false).unwrap();
    writeln!(out, "Rust no_std / Limpet presentation").unwrap();
    out.reset_colors().unwrap();
    writeln!(out, "{}", report.trim_end()).unwrap();
    if out.status() == QueryStatus::Supported {
        writeln!(out, "Unicode: Zażółć gęślą jaźń").unwrap();
    }
    for text in ["progress 1/3", "progress 2/3", "progress 3/3"] {
        out.progress(text).unwrap();
        aros::delay_ms(250);
    }
    if capabilities.can_rewrite_line() {
        writeln!(out).unwrap();
    }
    aros::delay_ms(2000);
    out.set_title("").unwrap();
    writeln!(out, "LIMPET_NOSTD_DONE").unwrap();
}
aros_main!(run);
