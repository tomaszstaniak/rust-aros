# Rust for AROS x86_64

Write AROS programs in Rust. This package contains everything needed to build
a Rust program on a PC or Mac and run it on AROS x86_64: a target definition,
a small runtime crate, a project template, a graphical example, and a helper
that deploys straight into a running AROS machine.

![The plasma example running on AROS](screenshot.png)

**Two ways to use it.** The `no_std` path (the runtime crate, template and
example below) is the stable one: `core` and `alloc`, `no_std` crates from
crates.io, and the AROS API through `extern "C"`.

There is also an **experimental `std`**: with the `aros-nightly` toolchain that
`std-groundwork/std-patch/make-std-toolchain.sh` creates, an ordinary Rust
program — `std::fs`, `std::thread`, `std::net`, `std::process`, `HashMap` — and
crates such as `serde`, `serde_json` and `anyhow` straight from crates.io
build and run on AROS. It has known gaps (bare program names in `Command`,
non-cryptographic randomness, `poll` on sockets only); they are listed in
`std-groundwork/std-patch/README.md`. See *Limitations* below before you
plan a project around either path.

## What is known to work

Verified on AROS One 1.3 (x86_64) under QEMU:

* `core` and `alloc` — iterators, `core::fmt`, `Vec`, `String`, `BTreeMap`
* crates from crates.io that support `no_std` — tested with `libm` and
  `hashbrown`
* floating point and 128-bit arithmetic, including the division behind
  decimal formatting
* calling the AROS API directly: `PutStr`, `Delay`, `AllocMem`/`FreeMem`
* calling the C library from the SDK, including varargs functions
* SDL2: the included example opens a window and animates a 320x200 plasma at
  **226 fps** inside emulation, without hardware acceleration

## Requirements

* An AROS x86_64 SDK and cross toolchain (`x86_64-aros-gcc`).
* Rust nightly with the sources:
  `rustup toolchain install nightly --profile minimal --component rust-src`
  Nightly is required because custom target files and `-Z build-std` are not
  stable features.

## Quick start

```sh
./setup.sh                    # finds your toolchain, writes x86_64-aros.json
cd template
cargo +nightly build --release
```

`setup.sh` looks for `x86_64-aros-gcc` on your `PATH` and for the SDK beside
it. If it guesses wrong, tell it:

```sh
AROS_GCC=/path/to/x86_64-aros-gcc AROS_SDK=/path/to/sdk ./setup.sh
```

The result, `target/x86_64-aros/release/hello-aros`, is an ordinary AROS
executable: copy it to your AROS machine and run it from a Shell.

### Optional: `cargo run` straight into QEMU

If you develop against AROS running in QEMU, `cargo run` can build, deploy and
start the program for you:

```sh
cd examples/plasma
cargo +nightly run --release
```

This uses `tools/aros-qemu-run.py`, which publishes the binary on a small ISO,
swaps it into the machine's CD drive and types the command into an open Shell.
It needs QEMU started with a monitor socket, a QMP socket and a CD drive:

```
-monitor unix:/tmp/aros-monitor.sock,server,nowait
-qmp     unix:/tmp/aros-qmp.sock,server,nowait
-drive   if=ide,index=2,media=cdrom
```

and an AmigaShell window open and focused in the guest. Socket paths, the
drive id and the volume name can be overridden with environment variables;
see the top of the script. It needs nothing but Python 3 and an ISO builder
(`hdiutil`, `xorriso`, `genisoimage` or `mkisofs`).

The result comes back as a screenshot rather than as text: AROS gives no way
to send output back to the host. Writing to a QEMU vvfat drive from inside the
guest looks like it works and silently produces a corrupt file, and `SER:`
never reaches QEMU's serial file, so neither is a usable channel.

## What is in the box

| Path | What it is |
|---|---|
| `setup.sh` | Detects your toolchain and writes the target definition. |
| `x86_64-aros.json.in` | Target definition template; see `TARGET-NOTES.md`. |
| `aros/` | The runtime crate: allocator, `println!`, panic handler, bindings. |
| `template/` | A minimal project to copy for your own program. |
| `examples/plasma/` | A graphical SDL2 example. |
| `tools/aros-qemu-run.py` | `cargo run` helper for a QEMU-hosted AROS. |
| `std-groundwork/` | Work towards a real `std`: `libc` bindings and their tests. |

## Writing a program

```rust
#![no_std]
#![no_main]

extern crate alloc;
use aros::{aros_main, println};

fn run() {
    println!("hello from Rust on AROS");
}

aros_main!(run);
```

The SDK's C startup still runs the process; `aros_main!` exposes your function
to it. The `aros` crate installs the allocator and the panic handler, so
nothing else is needed.

Memory from exec.library comes with its pairing rule enforced by the compiler:

```rust
let mut buf = aros::exec::Mem::new(1024).unwrap();
buf[0] = 42;
// FreeMem happens on drop, with the size it was allocated with
```

## Limitations

* **`std` is experimental.** It passes its own tests and runs `serde`, but it
  has not yet carried a large real program; `ripgrep` and `tokio` are the
  next things to try. The `no_std` path is the one to rely on today.
* **Nightly only**, and nightly moves; a future release may need the target
  file adjusted.
* **Safety stops at the API boundary.** Calls into AROS are `unsafe` FFI like
  any other C interface, and AROS shares one address space between all tasks.
  Rust protects the logic you write, not the system you call.
* **`panic = abort`.** There is no unwinder, so a panic prints and exits.
* **Do not `strip` the executable.** A full strip leaves an AROS binary whose
  relocations the loader silently skips, and it crashes before `main`. Use
  `--strip-unneeded --remove-section .comment` if you must; the saving is
  small because the relocations need the symbols.
* **The console is not UTF-8** — keep printed text to ASCII.
* Binaries are larger than the C equivalent, and they cannot be fully
  stripped, so expect a few hundred kilobytes for a small program.

## `std` — experimental, but working

`std-groundwork/` contains a patched `std` for AROS and the `libc` crate it
stands on. With the `aros-nightly` toolchain that `std-patch/make-std-toolchain.sh`
creates, an ordinary Rust program — `std::fs`, `std::thread`, `std::net`,
`std::process`, `HashMap`, and crates such as `serde` and `anyhow` straight
from crates.io — builds and runs on AROS. See `std-groundwork/std-patch/README.md`
for setup and the list of known gaps. The `no_std` path described above
remains the simpler, more robust option for now.

## What is next

In order: build `ripgrep` on the `std` toolchain as the first real-world test
(files, directories, arguments, threads, errors); try `tokio`, which will
show what `poll` over sockets is worth; then wire the SDL3 that AROS contrib
already carries (3.4.12, with SDL3_image and SDL3_ttf) to the `sdl3` crate.
Separately, the runtime tests in `std-groundwork/` are worth turning into a C
conformance suite for the AROS maintainers: they show exactly where the SDK's
POSIX surface diverges from what portable code expects.

## Prior art and credit

James Knipping ported the Zed editor and the ferail file manager to **AROS
aarch64**, which is what showed that Rust on AROS was possible at all; those
run under Macaros, AROS hosted on Apple Silicon. That work is not public, and
this package was built independently for **x86_64**, which is where AROS
desktop users are.

## License

MIT — see `LICENSE`.
