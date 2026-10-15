# Rust AROS renewal: current state

Verified 2026-10-15. Implementation base: rust-aros `8d35fda`.

## Platform policy

AROS ABIv11 is primary; ABIv1/mainline is next. Architectures in scope are
x86_64 and arm64 through hosted Macaros. The local Macaros installation is not
yet reliable, so arm64 execution acceptance is pending. Native Raspberry Pi is
outside this project scope. Determine Macaros's actual ABI from its SDK.

Zed is a reference workload for platform quality, not a delivered port. Audit a
pinned Zed revision before committing to its dependencies: its async stack is
not exclusively Tokio, and its GPUI backend is a separate platform project.

## Completed first M0 slice

- Active work is in the original `AROS-dev/rust-aros`, branch
  `codex/rust-renewal-m0`. The first commit was transferred from the temporary
  local clone after obtaining write permission. That clone is no longer the
  active source. Shared sysroots remain unchanged.
- The std installer requires a new destination and a new local rustup name.
  It rejects shared names, existing destinations (including dangling links),
  registered names and compiler mismatches; failed construction is cleaned up.
- Base defaults to nightly-2026-08-31 and must identify itself exactly as
  `rustc 1.100.0-nightly (0dfb098f3 2026-08-31)`. An installed base can be
  explicitly selected with AROS_RUST_BASE, subject to the same check.
- Wrapper paths are shell-quoted as literal data. The review finding was
  reproduced by a failing argv test and fixed; tests also cover literal
  metacharacters in the base executable path.
- Seventeen host tests pass (installer, C-build/setup and entropy contract
  checks). Run `python3 -m unittest discover -s tests -v`.
- Existing no_std template and std-hello built offline. std-hello also built
  from a new isolated sysroot; serde's host proc macro compiled in that build.
- Both binaries executed on v11-1 with exit status 0. See
  [the scoped report](evidence/2026-10-15-m0/REPORT.md).

The shared aros-nightly still points to the original sysroot-8d35fda. New local
registrations are aros-m0-20261015 and aros-m0-verified-20261015, with sysroots
under AROS/local. No registration replaces the shared toolchain.

## Next acceptance criteria

The C build scripts now use explicit AROS_GCC/AROS_SDK, track environment
changes, and accept AROS_AR. The GCC fallback replaces only the executable
suffix, not occurrences in parent paths. setup.sh persists the chosen archiver.
A real offline v11 libc-test compile/link passed in the original checkout; this
new binary was not executed in the guest.

M0 remains open: centralize explicit per-profile configuration (compiler,
archiver, SDK, target, sysroot and build output), remove libc-aros's
hardcoded C-library choice, and exercise threads, processes and
network-up/down cases on an explicitly matched v11 SDK/guest. The fresh run here
used the older AROS One v11 profile, not the 2026.09 release profile.

The first M1 entropy slice now reuses arospkg's pinned jitterentropy/platform
code with real failures propagated. Fresh x86_64/v11 acceptance covers 12 libc
checks, concurrent source initialization/readers, repeated forced initialization
failure, rebuilt std HashMaps/threads, and getrandom 0.3.4/rand 0.9.5 with an
explicit application custom backend, including uninitialized output buffers. See
[evidence](evidence/2026-10-15-entropy/REPORT.md).

Next: system-source selection for v1/Macaros, further getrandom versions only
as required by pinned consumers, a reusable application backend if additional
consumers need it, and the next terminal input/process milestones. Host rust-objcopy/libLLVM loading
produced a nonfatal warning and remains a host-toolchain follow-up.

Reuse Limpet SDK 0.3.0 and inspect aros-xterm/tools/rust-tty before adding terminal
adapters. Mainline Ratatui/Crossterm acceptance already exists with an isolated
experimental posixc runtime; this does not establish stock v11 compatibility.

## Limitations

No fresh v1, arm64/Macaros, async networking, HTTPS, watcher, SQLite or GPUI
acceptance in this slice. Source here now uses jitterentropy; existing shared
sysroots still contain their older implementation. The new aros-jent-20261015
registration is separate. Exact upstream v11/v1 default-branch revisions were
read successfully with gh API during M1; see the scoped report. Runtime
coverage does not transfer between profiles.

## Limpet presentation slice (2026-10-15)

The separate allocation-free aros-limpet crate supports std/no_std without
installing an allocator or panic handler. It reuses SDK 0.3.0 for capability
queries, geometry, colours, titles and progress output. Six Rust host tests pass
in addition to the 17 Python tests. Eight native cases on x86_64/older v11
passed with rc 0: both runtime styles in CON:, files redirected from CON:,
Limpet 0.4.1, and files redirected from Limpet. Files contain no ESC bytes.
See [the report](evidence/2026-10-15-limpet/REPORT.md) for binaries, screenshots
and the explicit unsafe handle-lifetime/task-access constructor contract.
Raw input, PTY/termios and Ratatui remain outside this completed slice.

## Process execution and transport (2026-10-15)

The initial all-spawn-fail diagnosis was corrected: payload copying lost the
execute bit. Shell launched the file, but exec rejected it. The staging helper
now preserves 0755, explicitly protects the RAM: copy and records permissions.
Rust/C pipe/fcntl/errno probes agree; the suspected ABI mismatch was not found.

A narrow older-CRT execvp adapter preserves empty arguments, checks qualified
paths before takeover can lose errno, and retains honest failures. Native
x86_64/older v11 acceptance passes leading/consecutive/trailing empty, spaced,
quoted, UTF-8 and star arguments; eight missing-file errors followed by a valid
spawn; try_wait/EOF/cached wait; 131072 bytes each of stdout/stderr with child
exit 9; and eight spawns during 977 allocation iterations in a second thread.
Child::kill reaches ENOSYS, with a naturally exiting child reaped.

Two bytewise framing cases pass: two UTF-8 messages with separate stderr,
stdout EOF and child exit 7, plus rejection/cleanup after a bad response.
Ordinary 8 KiB Stdin/BufReader framing still blocks on this PIPE handler's
read-until-requested-length/EOF behavior. The explicit bytewise fixture is a
workaround, not acceptance of generic buffered LSP/stdin or a real server.

The permission-denied fixture passes on native RAM:. Its initial permission
setup on RESULTS: FAT failed and is retained as a separate transport-volume
limitation. Combined native matrix: nine passes and one unsupported kill.
Host regression: 23 Python tests, six Limpet tests, five framing tests and one
integration test with 12 host process cases pass. New isolated production
toolchain: aros-exec-20261015. Shared sysroots are unchanged.
See [corrected scoped evidence](evidence/2026-10-15-process-exec/REPORT.md).

The reusable transport and real pinned language-server slice is now accepted
below. General
buffered stdio on DOS PIPE needs its own runtime design. Keep cancellation,
process signals, general PATH/environment/cwd behavior, v11 2026.09, v1 and
arm64/Macaros as separate unaccepted criteria.


## Reusable transport and real language server (2026-10-15)

aros-lsp-transport now provides bounded Content-Length frames, explicit Buffered
and DosPipe policies, clean EOF, terminal error state, partial-write completion
and per-frame flush. Seven host tests pass; process-stdio reuses the parser and
retains its five framing tests and 12-case process integration. Python
regressions remain 23/23 passing.

A pinned upstream wat_server 0.11.0 now runs natively on the matched older
x86_64 ABIv11 profile. Actual acceptance passes pre-main inventory registration,
initialize, invalid WAT diagnostics, Unicode document repair and diagnostic
clearing, formatting edits interpreted with UTF-16 positions, document symbols
handled by worker threads, shutdown, EOF, successful child wait and joined empty
stderr. The same client passes against original and patched host servers.

The first native run exposed inventory's missing AROS constructor cfg: Salsa
panicked on an unregistered document ingredient. A checksum-pinned, versioned
patch selects .ctors for x86_64 AROS, matching GCC 10.5. Sources in the upstream
export and Cargo registry remain untouched; dependency preparation is atomic
and verifies patched contents on reuse. Application dependencies explicitly use
libc-aros, not the unsupported crates.io libc target.

See [scoped evidence](evidence/2026-10-15-wat-lsp/REPORT.md) and
[reproduction](../ports/wat-server/README.md). This is real synchronous LSP
functionality on v11, using the explicit DosPipe workaround. It does not close
general buffered PIPE, cancellation/signals, async I/O, filesystem watching,
rust-analyzer, Zed/GPUI, newer v11 profiles, v1 or arm64/Macaros acceptance.

Next: promote this tested build/profile wiring beyond the WAT recipe, then
accept concurrent request/cancellation behavior and a consumer requiring async
readiness. Keep hosted arm64 blocked on a working Macaros environment rather
than treating cross-linking as runtime evidence.
