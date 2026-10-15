# M0 first-slice build and runtime checks

Date: 2026-10-15. Base source: rust-aros 8d35fda plus the installer and docs
changes in this branch. No runtime source was changed for these binaries.

## Address

- Target: x86_64 AROS ABIv11, older AROS One 1.3 profile.
- Compiler: AROS/toolchain/x86_64-aros-gcc, GCC 10.5 toolchain.
  Driver SHA-256: c2783f9b3594f7ccab0ebf17f025ba715db958c7079ddd84f8fceed6abac649a.
- SDK: AROS/sdk (existing older v11 SDK). This is not the v11 2026.09 SDK.
  SDK content manifest was not captured in this slice; exact clean rebuild
  provenance remains an M0 follow-up.
- Rust: 1.100.0-nightly (0dfb098f3 2026-08-31), installed nightly base selected
  explicitly and version-checked; std built from aros-m0-20261015's new sysroot.
- Guest: pool v11-1, QEMU pc x86_64/TCG, 1024x768 display. Reserved exclusively
  as codex-rust-renewal-20261015, stopped, collected and released after the run.
- Input: immutable POOLINPUT ZIP, unpacked into RAM:rust-m0. No system files
  changed. std-hello created, read and deleted RAM:machine.json.

## Build commands

From the isolated checkout:

```sh
AROS_GCC=/Users/tumash/Work/AROS/toolchain/x86_64-aros-gcc \
AROS_SDK=/Users/tumash/Work/AROS/sdk sh setup.sh
cd template
cargo +nightly build --release --offline
```

Here the installed nightly was checked to be the exact pinned version above.
The installer accepts this offline selection:

```sh
AROS_RUST_BASE=nightly sh std-groundwork/std-patch/make-std-toolchain.sh \
  /Users/tumash/Work/AROS/local/rust-aros-m0-sysroot aros-m0-20261015
cd std-groundwork/std-hello
CARGO_HOME=/Users/tumash/Work/AROS/local/rust-aros-cargo \
CARGO_TARGET_DIR=/Users/tumash/Work/AROS/local/rust-aros-build/m0-std \
cargo +aros-m0-20261015 build --release --offline
```

The local Cargo home uses a copied index and the existing read-only crate cache;
crate source extraction stays local. The first attempt using the shared Cargo
home failed because sandbox permissions disallowed extracting anyhow there.
The first installer attempt likewise failed at rustup registration; its new
sysroot was cleaned up. Registration succeeded with reviewed host permission.

## Runtime results

| Binary | SHA-256 | Observed result |
| --- | --- | --- |
| hello-aros | 543ebc3b9b3c83152541245d7ec5ae9148af8e2d73b0a7db96934d0f43ef5dce | rc 0; Vec/String formatting, 1024-byte AllocMem buffer, first byte 42 |
| std-hello | 99fdd05634440f8a63bf8396aaf5677d3b10f3ac5c7040b2ad9ce42810b4e9f1 | rc 0; serde JSON filesystem round-trip, anyhow error formatting |

Raw output and exit codes are the adjacent rust-m0-20261015-*.txt files. The
completion marker is separate. Pool collection hashed files against the stopped
read-only results image; its full manifest remains in AROS/local/rust-aros-m0-results.

## What this did not establish

This was not a fresh validation of threads, clocks, subprocess pipes, networking,
cryptographic randomness, or the complete std surface. No SDK link trace was
captured. It does not transfer to v11 2026.09, v1, or hosted arm64 Macaros.
It does not establish Zed or any GPUI backend builds or runs. The installer
tests use a synthetic rustup for failure injection; separate real creation
checks validate the known local base and successful registration path only.
