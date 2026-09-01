//! Plasma — a small graphical demo for AROS x86_64, written in Rust.
//!
//! Renders a 320x200 buffer and lets SDL scale it into a 640x400 window.
//! AROS draws SDL in software, so the work per frame is kept deliberately
//! small: three table lookups per pixel and a palette indirection.
//!
//! Press any key or close the window to quit; it also stops on its own after
//! a while so that `cargo run` always returns.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use aros::sdl::Screen;
use aros::{aros_main, println};

const W: usize = 320;
const H: usize = 200;
const RUN_MS: u32 = 15_000;

/// 512-entry sine table scaled to 0..=63, so four of them fit in a byte.
fn sine_table() -> Vec<u8> {
    (0..512)
        .map(|i| {
            let a = (i as f64) * core::f64::consts::TAU / 512.0;
            ((libm::sin(a) * 0.5 + 0.5) * 63.0) as u8
        })
        .collect()
}

/// A cyclic rainbow, built from the same sine table.
fn palette(sin: &[u8]) -> Vec<u32> {
    (0..256)
        .map(|i| {
            let c = |phase: usize| -> u32 {
                let v = sin[((i * 2) + phase) & 511] as u32; // 0..63
                (v * 4).min(255)
            };
            0xFF00_0000 | (c(0) << 16) | (c(170) << 8) | c(340)
        })
        .collect()
}

fn run() {
    println!("plasma: Rust on AROS x86_64 - press a key to quit");

    let sin = sine_table();
    let pal = palette(&sin);
    let mut frame: Vec<u32> = vec![0; W * H];

    let mut screen = match Screen::open(b"Plasma (Rust)\0", W, H, 640, 400) {
        Ok(s) => s,
        Err(e) => {
            println!("plasma: {} ({})", e, aros::sdl::error());
            return;
        }
    };

    let start = aros::sdl::ticks();
    let mut frames: u32 = 0;

    loop {
        let t = aros::sdl::ticks().wrapping_sub(start);
        if t >= RUN_MS || screen.should_quit() {
            break;
        }

        // Two moving phases, so the pattern never repeats exactly.
        let p1 = (t / 12) as usize;
        let p2 = (t / 20) as usize;

        for y in 0..H {
            let ry = sin[(y * 3 + p2) & 511] as usize;
            let row = y * W;
            for x in 0..W {
                let v = sin[(x * 2 + p1) & 511] as usize
                    + ry
                    + sin[((x + y) * 2 + p2) & 511] as usize
                    + sin[((x + 512 - y) * 3 + p1) & 511] as usize;
                frame[row + x] = pal[v & 255];
            }
        }

        screen.present(&frame);
        frames += 1;
    }

    let elapsed = aros::sdl::ticks().wrapping_sub(start).max(1);
    println!(
        "plasma: {} frames in {} ms ({}.{} fps at {}x{})",
        frames,
        elapsed,
        frames * 1000 / elapsed,
        (frames * 10000 / elapsed) % 10,
        W,
        H
    );
}

aros_main!(run);
