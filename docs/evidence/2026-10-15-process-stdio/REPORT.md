# Process transport acceptance, 2026-10-15

This slice adds a portable std::process fixture and fixes two concrete SDK
mismatches. It does **not** establish working Command/LSP transport on AROS.

## Profile and reproduction

x86_64 ABIv11, older AROS One 1.3 UP guest v11-1, QEMU pc/TCG, qemu64,
2 configured vCPUs, 2 GiB; AROS/toolchain GCC 10.5 and older AROS/sdk.
Not the v11 2026.09 profile. Rust 1.100.0-nightly
(0dfb098f3 2026-08-31), offline Cargo cache. Baseline toolchain:
aros-jent-20261015. Final toolchain: separate aros-process-20261015,
sysroot AROS/local/rust-aros-process-sysroot. Shared toolchains untouched.

Create a new toolchain from the repository root:

```
AROS_RUST_BASE=nightly sh std-groundwork/std-patch/make-std-toolchain.sh /new/sysroot new-local-name
```

The installed nightly must match the exact pinned version. Build from
std-groundwork/process-stdio with cargo +new-local-name build --release
--offline. CARGO_HOME was AROS/local/rust-aros-cargo; final target directory
AROS/local/rust-aros-build/process-fixed. Host tests run from the repository
root with cargo +stable test --manifest-path
std-groundwork/process-stdio/Cargo.toml --offline (host cwd is intentional).

Exclusive owner codex-rust-renewal-20261015. Each attempt used a separate
stop-stage-start cycle, extracted only into RAM:pr, wrote unique RESULTS:
files, stopped before collection, then released. No guest libraries, startup
files or settings changed. Full hashed collection manifests remain under
AROS/local/rust-process-results-{first,second,fixed}. This directory preserves
only relevant guest-created reports, exit codes and screenshots.

## Results

Host: 20 Python regression tests, six Limpet Rust tests, five framing unit
tests and one integration test exercising eight real process cases all pass.
The integration test covers status decoding, UTF-8/quoted/empty arguments,
eight missing executable errors followed by a valid spawn, try_wait/wait,
kill, 131072 bytes each of stdout/stderr, two fragmented protocol frames,
and invalid-response cleanup. These are host results only.

| Native attempt | Result |
| --- | --- |
| First, baseline sysroot | args failed byte round trip; spawn-error failed error-kind check; wait started without a final report before the run was stopped; remaining cases not reached |
| Second, more detailed reports, same baseline sysroot | args, spawn-error, kill and wait all returned ENOSYS (78); wait did not reach after-spawn; all four returned rc 20 |
| Final, corrected status/CLOEXEC sysroot | status decoding passed, rc 0; all seven spawn-dependent cases returned rc 20, OS error 0; script completed |

The early console screenshot showed only RUN-args, but collected files prove
that attempt subsequently advanced to wait. It is not evidence that args hung.
The first incomplete wait is also not isolated proof that try_wait blocks.
The second and final runs returned; the wait checkpoint localizes their error
to spawn, before try_wait/closing stdin. Cross-run differences remain unresolved.
The kill case never reached Child::kill in the final run, so no native kill
support or unsupported classification is established by this slice.

## Corrections and remaining diagnosis

AROS posixc sys/wait.h defines WEXITSTATUS as status & 0xff and WIFEXITED as
true. The old bindings used BSD wait encoding and could report wrong exit
codes/signals. All W* helpers now match the selected SDK's available states.
The status fixture verifies std ExitStatus::from_raw for 0,1,7,9,20,127,255
on the final native binary; it does not verify an actual child exit.

std FileDesc previously selected ioctl(FIOCLEX). The selected AROS posixc
source implements FD_CLOEXEC with fcntl(F_GETFD/F_SETFD), while its ioctl only
supports TIOCGWINSZ. The patch now selects std's existing fcntl branch in both
complementary cfg lists. Host regression compiles the selection for AROS,
Haiku and an unrelated target, and rejects a changed source layout. This
corrects API selection; closure across exec remains unverified.

Next isolate raw pipe/fcntl/dup2 calls and compare Rust FFI with a fixed-arity
C bridge compiled by the same GCC. The linked binary contains AROS varargs
wrappers (__fcntl_CrtBase_wrapper), so calling convention and errno propagation
need direct checks. Review the legacy fork alias to vfork: a generic Rust
fork/exec path is not an accepted process backend. Do not infer the root cause
solely from error 0, and do not emulate termination using RemTask.

## Limits

This is bounded Content-Length echo framing, not a language server or a full
LSP implementation. Reads/writes/waits have no internal deadline; guest runs
were externally observed and stopped. No cancellation/timeout, leak absence,
PTY, async transport, watcher, networking, v1 or arm64/Macaros acceptance.
The native failures are deliberately retained as regression evidence rather
than counted as passing capabilities. Binary hashes identify each payload;
the final source adds only Cargo check-cfg metadata after the final build.
