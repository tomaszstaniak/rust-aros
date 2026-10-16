# Stepped verified TLS on x86_64 ABIv11

Verified 2026-10-16. Accepted final runs: 10-abort and 11-unwind only.

## Platform and inputs

Older AROS One v11-2 under QEMU, x86_64 ABIv11; GCC10.5 with
/Users/tumash/Work/AROS/sdk (libcrt SHA in each build manifest), Rust
1.100.0-nightly (0dfb098f3 2026-08-31), private aros-net-20261016 sysroot.
No shared SDK, toolchain, guest library or trust-store setting changed.
VM input was staged stopped, extracted to RAM:rh5, reports collected stopped
and checked against the controller's SHA manifest; reservation released.

Mbed TLS3.6.7 and jitterentropy3.7.0 come from a read-only Git archive of apkg
91ac4d8f00bbaf5ac5b057322d3937a61acd1cd7. Checked apkg HEAD661a8c6 has no
changes to these TLS/SDK/third-party inputs. The build reads the archived SDK
source lists and uses its flags: jent -O0/-fwrapv, Mbed/adapter -O2. Existing
older shared v11 OpenSSL is not used; no v1 binary is linked. The host TLS
fixture uses OpenSSL4.0.3/Python ssl, private generated test keys and explicit
CA files. Keys remain host-local and were never staged or committed.

## Accepted behavior

Both abort and unwind have complete start/end UUID, zero shell statuses and
passed tls-check.py results in validation-{abort,unwind}.json. Source/header
maps contain the same523 inputs in both profiles plus the opaque adapter;
binary hashes, build commands, lock hash and compiler identity are retained.
Four foreign constructors are refused with ownership domain4 while an owner
TLS context is live; the owner completes the exchange afterwards.

Per profile, fixture counts are10 accepted TLS connections/errors[]:

- Four exact HTTP exchanges, required CA/name/date verification and authenticated
  TLS close_notify. Fixture records TLS1.3. Two exchanges run concurrently as
  logical futures, exercising global PSA lifetime until the last context drops.
- Wrong name: certificate verify flag0x04. Expired leaf:0x01. Other trust root:0x08.
  All three fail before HTTP and match fixture certificate alerts.
- Pending handshake cancellation after the peer receives224 ClientHello bytes,
  then established-context drop and another successful connection.
- Transport truncation without close_notify produces transport error domain5,
  even when an HTTP body was received.

The separate forced-entropy binary fails before connect with domain3,
PSA insufficient entropy (-148), jitterentropy startup status-2. The fixture
monitors port18773 through completion of both guest binaries and observes zero
connections. There is no fallback source or insecure verification switch.

The opaque C API advances a single handshake state and reports WANT_READ/
WANT_WRITE to async-io; successful intermediate states yield to the owner
reactor. Read/write/close step without blocking retry loops. Drop frees only;
explicit async close handles the alert. TLS objects are !Send/!Sync. Guarding
foreign construction precedes all mutable owner-only TLS global reads.

## Failures caught before acceptance

Diagnostic06 is excluded, despite a guest PASS: its fixture observed a TCP
close before any ClientHello. Randomized future::race could start a500ms timer
before synchronous crypto initialization. Construction/TCP begin is now split
from handshake; cancellation timing starts afterward with handshake-first
future::or and a2sec timer. The fixture's ClientHello assertion was retained,
not weakened. Both final runs satisfy it.

Review found missing actual patched dependency/header hashes. The final build
now records all four patched Cargo trees and the copied include directory;
tests reject modified dependency/header data. Review also found a foreign
constructor reading live before the owner guard. A host test runs the actual
C condition: RED exit7 (observed global read), GREEN exit0 after reordering.
Native foreign-constructor gates then pass. Earlier prototypes are not the
accepted final source or substituted for final runs.

Host suite:43 tests pass (host-tests.log). Stale UUID, missing certificate case,
forbidden entropy connection and changed compiled sources are rejected by
report-checker tests. An existing build output is refused without changing its
manifest. All material review findings are resolved.

Existing native socket/entropy/process5/LSP7 regression evidence from the same
continuation remains in ../2026-10-16-real-async/. This TLS slice changes no
libc source; retained copied libc C/selector/Rust inputs match that accepted
boundary. These regressions were not rerun for each TLS-only rebuild.

## What this did not show

This is a bounded example transport on the owner task, numeric IPv4, explicit
CA and selected driver. It does not accept DNS, Unix sockets, worker-task I/O,
full smol, reqwest/rustls, WebSockets, HTTP2, proxies, general server TLS, a
system trust-store integration or Zed. TLS1.2 is configured but this runtime
fixture negotiated1.3; no TLS1.2 acceptance is inferred. No performance,
physical hardware, v1 or arm64/Macaros result is claimed.

Crypto and entropy startup remain synchronous CPU work on the owner task;
an async timer cannot preempt a C call. The copied gmtime adapter assumes
serial owner-task TLS calls. Global PSA lifetime is coordinated only among
these adapter contexts, not arbitrary unrelated PSA users. Cancelling a write
and reusing that context is not accepted; cancelling construction/handshake
here destroys the context. This binary example is not a published reusable SDK.

A fresh output/run UUID and strict reports improve repeatability; shared SDK/
toolchain contents are not fully vendored, the build is not claimed hermetic,
and guest binary bytes were not independently rehashed. The combined one-command
socket/async/TLS/process/LSP acceptance recipe remains Task4 work; current TLS
prepare/build/payload/fixture/check commands are documented in
../../../ports/async-network/README.md.
