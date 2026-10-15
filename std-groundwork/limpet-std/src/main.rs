use aros_limpet::Output;
use core::fmt::Write;
fn main() {
    // This single-task example keeps its process-owned output open.
    let mut out = unsafe { Output::current() };
    let report = format!(
        "STD query={:?} geometry={:?} rewrite={} ioerr={}\n",
        out.status(),
        out.capabilities().geometry(),
        out.capabilities().can_rewrite_line(),
        out.io_error()
    );
    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(path, &report).unwrap();
    }
    out.set_title("Rust std / Limpet").unwrap();
    out.set_ansi(6, false).unwrap();
    writeln!(out, "Rust std / Limpet presentation").unwrap();
    out.reset_colors().unwrap();
    write!(out, "{report}").unwrap();
    out.progress("std progress complete").unwrap();
    if out.capabilities().can_rewrite_line() {
        writeln!(out).unwrap();
    }
    std::thread::sleep(std::time::Duration::from_secs(2));
    out.set_title("").unwrap();
    writeln!(out, "LIMPET_STD_DONE").unwrap();
}
