//! Starting point for a Rust program on AROS x86_64.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use aros::{aros_main, println};

fn run() {
    println!("hello from Rust on AROS x86_64");

    // `alloc` works, so the usual collections are available.
    let mut names: Vec<String> = Vec::new();
    for who in ["Amiga", "AROS", "Rust"] {
        names.push(String::from(who));
    }
    println!("{} words: {}", names.len(), names.join(", "));

    // exec.library memory, freed automatically with the right size.
    if let Some(mut buf) = aros::exec::Mem::new(1024) {
        buf[0] = 42;
        println!("exec.library gave us {} bytes, first={}", buf.len(), buf[0]);
    }
}

aros_main!(run);
