//! Ordinary Rust executable: CRT calls rustc's main -> lang_start -> this main.
use std::process::ExitCode;

struct MainDrop;
impl Drop for MainDrop {
    fn drop(&mut self) { println!("Cargo main: local destructor ran"); }
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "lang_start must initialize arguments");
    assert_eq!(args[1], "runtime-argv");
    let _drop = MainDrop;
    if args[2] == "exit7" {
        println!("Cargo main: returning ExitCode 7");
        return ExitCode::from(7);
    }
    assert_eq!(args[2], "workload");
    aros_acceptance::probe("RAM:rust-cargo-main.txt");
    println!("Cargo main: returning ExitCode 0; awaiting shell receipt");
    ExitCode::SUCCESS
}
