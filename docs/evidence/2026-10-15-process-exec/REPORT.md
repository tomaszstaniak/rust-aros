# Process execution and framed transport, 2026-10-15

## Corrected diagnosis

The earlier [report](../2026-10-15-process-stdio/REPORT.md) recorded seven
spawn-dependent failures. Their common cause was a staging mistake:
shutil.copyfile created a non-executable 0644 payload. Shell launched it, but
exec rejected it. This was not evidence of a Rust/posixc ABI mismatch.

The permissions/ reports show the same binary before and after only Protect +e
on RAM:pr/process-stdio: ---rw-d became ---rwed. Command then executed the child.
Its argument report differed only by the missing empty argument. wait/try_wait/
EOF completed, and kill reached the API, reporting ENOSYS and reaping the
naturally exiting child. Immediate screenshots sometimes lagged execution;
collected final reports determine these results.

## Scope and reproduction

x86_64 ABIv11, older AROS One 1.3 UP pool v11-1, QEMU pc/TCG, qemu64,
2 configured vCPUs, 2 GiB, GCC 10.5 from AROS/toolchain with AROS/sdk.
Not the v11 2026.09 profile. Rust 1.100.0-nightly (0dfb098f3 2026-08-31).
Offline Cargo cache: AROS/local/rust-aros-cargo.

Baseline toolchain: aros-process-20261015. Final production toolchain:
aros-exec-20261015 at AROS/local/rust-aros-exec-sysroot. Build from
std-groundwork/process-stdio with cargo +aros-exec-20261015 build --release
--offline; CARGO_TARGET_DIR was AROS/local/rust-aros-build/process-exec.
Create a fresh toolchain with std-patch/make-std-toolchain.sh and explicit
AROS_RUST_BASE=nightly, checked against the exact pinned compiler.
After editing rust-src, use a new/clean target directory: Cargo reused the old
std build until the diagnostic output directory was cleaned.

Use process-stdio/tools/stage-payload.py to make a new input directory.
It preserves metadata, forces 0755 and writes a guest script with Protect +e
on the RAM: copy plus List evidence. It refuses existing destinations and
validates report prefixes. Stage/start only with an owned VM reservation,
unpack POOLINPUT:PAYLOAD.ZIP into RAM:pr with redirected UnZip log, inspect
extraction and Execute RAM:pr/run-tests. Collect while stopped and release.

Owner: codex-rust-renewal-20261015. Raw hashed collections:
AROS/local/rust-process-results-{abi,cspawn,trace,executable,ex-baseline,final,denied}.
Only scoped reports are retained here; binaries.sha256 identifies each payload.
No shared toolchain, guest library, startup file or setting was replaced.

## Boundary investigation

- ffi/: direct Rust and GCC fixed-arity bridges agree on pipe creation,
  fcntl get/set/clear FD_CLOEXEC, close and invalid-fd errors. Both errno
  readers agree, including EBADF=9. The suspected varargs/errno mismatch
  was not reproduced for these calls.
- cspawn/ and permissions/: C vfork+execv and vfork+execvp also failed on the
  non-executable payload. A pid and wait alone did not prove exec. The later
  exec_return=-1 measurement corrected the initial interpretation. Parent
  errno retained EACCES=13, post-exec child errno was 0, failed-child status 0.
- trace/: a disposable aros-process-trace-20261015 sysroot reached every
  do_exec phase, including successful SIGPIPE reset (previous handler 1).
  execvp returned with errno 0. Instrumentation changes timing/stack layout;
  the production sysroot has none. Trace filenames encode the phase as the
  first two of their three trailing digits, e.g. 010 is phase 1.
- buffered-baseline/: execute permission correct, output returned 131072
  A bytes and 131072 B bytes, actual child status 9. Ordinary lsp started
  without a final report before external stop; later cases were not reached.

## Production changes and rationale

libc's execvp binding enters a narrow C adapter for this older CRT. Empty
arguments become the DOS command-line representation "" without modifying
caller argv. Qualified paths (slash/colon, matching the SDK rule) are checked
with stat for existence, directory type and execute bits before CRT takeover
can lose errno. Bare names retain SDK PATH lookup. A returning exec without
errno remains a failure, mapped to EIO. Precise race/PATH failure errors are
not claimed; preflight cannot remove TOCTOU.

Allocation occurs only for empty entries. AROS explicitly permits child
malloc/free in its own CRT heap, removed on _exit or transferred through exec:
local compiler/crt/__crt_vfork.c, extra-functionality documentation, and
compiler/crt/posixc/__vfork.c. SDK argument escaping itself allocates.
This is not a portable POSIX post-fork allocation claim.

The selected PIPE handler completes reads at requested length or EOF
(workbench/fs/pipe/pipesched.c). Default 8 KiB Stdin/BufReader reads stall short
interactive messages. No global one-byte file-read shim was added. Separate
lsp-bytewise cases bypass the child's internal Stdin buffer with exclusively
owned fd 0 and one-byte framing buffers on both receivers; bodies use their
declared length. Ordinary buffered cases remain and are not accepted.

This matches the independent aros-xterm docs/o2-and-dos-stream.md audit,
local HEAD d8689f6227aeda370e03895778aac463c2d8ef42, document git object
c8f10788599c0aeb5e436cca71e4c20cdc319e4c. Its mainline runtime result does not
transfer to v11; the finding here also has native v11 blocked/bytewise evidence.

## Native results

| Case | Result |
| --- | --- |
| Raw ExitStatus decoding | pass, rc 0 |
| Leading/consecutive/trailing empty, spaced, quoted, UTF-8, star arguments | pass, rc 0 |
| Eight missing executable errors, then successful spawn | pass, rc 0 |
| try_wait while running, stdin EOF, repeated/cached wait | pass, rc 0 |
| stdout/stderr flood | pass, 131072 bytes each, child status 9, rc 0 |
| Spawn during allocation in another thread | pass, eight children, 977 allocation iterations, worker joined, rc 0 |
| Child::kill | unsupported, ENOSYS=78, natural exit 0 and reaped, report rc 0 |
| Bytewise frame transport | pass, two fragmented UTF-8 frames, separate diagnostics, EOF, child status 7, reader joined, rc 0 |
| Invalid bytewise response | pass, rejected, child status 7, reader joined, rc 0 |
| Non-executable fixture on RAM: | pass, PermissionDenied, rc 0 (supplement) |

The initial RESULTS: FAT permission setup failed with OS error 0, rc 20.
Its raw report is retained; moving only the fixture file to native RAM:
resolved preparation and exercised PermissionDenied. This does not establish
permission support on the FAT exchange volume. The supplement's rebuilt
binary changes only that fixture's path/error context; production libc and
the other nine cases are identical to the main matrix binary. Both hashes
are recorded. Combined scoped result: nine passing cases and one honest
unsupported kill result.

SCRIPT-RETURNED is not a blanket pass. The failed FAT preparation remains
visible alongside its successful RAM: correction.

## Host checks and limits

23 Python tests pass, including injected exec missing-file/directory/permission/
allocation/missing-errno and multiple-empty/no-empty cases, plus payload metadata,
safe prefixes and overwrite refusal. Six Limpet tests, five framing unit tests
and one integration test with 12 real host process cases pass. The empty-argument
contract failed on direct execvp, then passed with the adapter.

No arbitrary process-safety, leak-absence, general environment/cwd/pre_exec,
cancellation or timeout acceptance. This is an echo transport fixture, not a
language server. No v1, arm64/Macaros, PTY, async transport or generic buffered
stdin fix. The adapter targets this older SDK encoding and needs separate
selection/validation for other profiles. The nonfatal host objcopy/libLLVM
warning recurred; build/link succeeded, host packaging remains open.
