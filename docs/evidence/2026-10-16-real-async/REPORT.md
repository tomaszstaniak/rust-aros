# Bounded real async networking acceptance, 2026-10-16

Accepted on x86_64 ABIv11, older AROS One v11-2 under guarded QEMU, GCC10.5,
AROS/sdk and private aros-net-20261016 Rust 1.100.0-nightly (0dfb098f3).
Both abort and unwind builds pass. No new HTTPS, v1, arm64/Macaros,
full smol, general executor or Zed runtime result is claimed.

## What now runs

This is the real published async-net2.0.0 over async-io2.6.0 and polling3.11.0,
with futures-lite2.6.1 and a separate locked consumer graph. Exact archive
checksums and Zed reference revision/lock are versioned in ports/async-network.
It is not the complete Zed dependency graph. The unmodified path failed in
rustix with 395 diagnostics; the target-specific adapter excludes that backend
on AROS rather than inventing missing libc APIs/constants to silence it.

The C selector uses the accepted tagged descriptor boundary, one AllocSignal
bit per selector and WaitSelect's signal mask. It does not clear pending
notifications before waiting. Only notification and deferred retirement run
from other tasks. Registration/waiting stay on the opening task. The async-io
owner driver neither spawns a helper thread nor hands the reactor to one.
Async handles are !Send/!Sync; the compile-negative proof rejects an actual
async-net TcpStream with a Send bound because of the ownership marker.

## Native assertions

The final unwind run is rust-async-03.log; the independent default-abort build
is rust-async-04.log. Each fixture records 87 connections, a final done request
and errors[]. Both native programs reach the final PASS marker. They exercise:

- 80 selector create/drop lifecycles,40 foreign retire/reap cycles;
- 32 notifications racing before/during wait, separate-selector signal isolation;
- oneshot rearm and level repeats, two ready level sockets with event capacity 1;
- foreign public wait/modify/delete returning Unsupported before locking;
- 32 cross-thread future wakes, every test worker joined;
- real asynchronous delayed/fast TCP, local clones and concurrent logical tasks;
- a 20ms timer while another future waits for a 250ms response;
- cancelled read future/drop followed by 80 successful socket lifecycles;
- 16 checked refused connections and EOF;
- explicit unsupported DNS and IPv6 paths.

The native app has a 30-second overall asynchronous deadline; fixture accepts
have a 60-second deadline, socket requests a 3-second timeout. Standalone selector
waits are finite. All host fixture workers terminate and the fixture exits0.
The accepted driver is async_io::block_on on the owner; an arbitrary executor
or worker-thread network model is not implied by these results.

## Review findings reproduced and fixed

An independent read-only review found two issues. The real backend compiled
against a synthetic readiness syscall delivered [1,1,1,1] instead of
[1,2,1,2] with two always-ready level sources and capacity 1. That regression
failed before rotating delivery was added, then passed. The native small-event
buffer case also passes. The public Poller wrapper now checks owner before its
try_lock; otherwise a foreign wait could skip the backend check when busy.
The native worker asserts rejection of public wait/modify/delete.

Signal retirement avoids reading the selector after Permit: the owner may
already have reaped it. Deferred signal bits are freed on owner operations or
library shutdown. The lifecycle boundary requires application notifier workers
to finish before shutdown; all workers in the accepted test join explicitly.

## Refusal fixture correction

The first prototype passed signal/wake/timer/TCP/cancel cases, then timed out
waiting for a refused connection. It is incomplete, not final acceptance.
The host fixture had bound a TCP socket without listen. On this macOS that
silently drops SYNs, also reproduced by a host client with a 300ms timeout.

A direct BSD probe on the same v11-2 profile showed read/write/exception all0,
SO_ERROR0 after 2 seconds against that bound endpoint. After closing it, the
same binary immediately returned read/write readiness, exception0, SO_ERROR61
(ConnectionRefused), in 49us and 11us. The correction is in the fixture: check
an actually closed endpoint before and after the test. No guest error-readiness
workaround was added. The C diagnostic source and both logs are retained here.
Its include order emitted prototype-visibility warnings; neither run is a
warning-free compile claim, and the observed primitive ABI calls are unchanged.

## Regressions and provenance

Fresh selector-capable libc sources match across both async builds and the
LSP build-owned copy. The existing socket boundary gate again passes 83 fixture
connections/80 lifecycles and file/ownership checks. Native getrandom/rand/four
HashMap worker regression passes. Five process gates pass: status,128KiB separate
stdout/stderr, arguments, spawn-error recovery and bytewise framing. All seven
existing unwind LSP cases pass their fresh build manifest/receipt validator.
This preserves that previously scoped LSP result without broadening its
natural-load/backpressure claims. Host suite: 37 tests pass.

Preparation checks original archive digests, rejects traversal/links/special
members/duplicate names, refuses existing output and builds offline/locked.
A repeat preparation against the accepted output failed without changing its
manifests or binary. Build-owned sources and SHA manifests remain under
AROS/local/rust-async/recipe-03 and recipe-04-abort; raw logs and stopped
collections are in the same local root. acceptance.json records hashes and
build metadata. Selected reports were rechecked against the stopped controller
collection manifest. Guest binary bytes were not independently rehashed.

All deployments followed stopped staging/private extraction/stopped collection.
v11-2 is stopped and released. No shared SDK, accepted private toolchain,
other project's source or persistent guest setting was changed. apkg HEAD had
advanced to 661a8c6; its TLS/sdk/third_party inputs remain unchanged from the
selected 91ac4d8 pin. Stepped Mbed TLS HTTPS remains the next task.
