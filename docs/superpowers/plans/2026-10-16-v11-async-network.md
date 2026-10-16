# v11 async networking and verified TLS implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Accept a bounded real async TCP consumer and certificate-verified HTTPS on the named x86_64 ABIv11 profile, with explicit ownership and failure semantics.

**Architecture:** Extend the existing libc-aros socket boundary only where native evidence requires it. Adapt exact published async-io/polling/async-net versions from a pinned Zed lock through versioned target-specific patches. Reuse copied/pinned apkg Mbed TLS sources through an opaque, stepped C interface on the socket-owning task. Keep unsupported thread/descriptor modes explicit.

**Tech Stack:** Rust 1.100.0-nightly (0dfb098f3), private std toolchain, GCC10.5 / AROS/sdk, async-io2.6.0/polling3.11.0/async-net2.0.0; Mbed TLS3.6.7 and jitterentropy3.7.0.

**Spec:** ../specs/2026-10-15-rust-aros-renewal-design.md, M3; user 2026-10-16 authorizes a larger autonomous slice and specifies Mbed TLS or separately rebuilt/bundled newer OpenSSL for v11.

## Global constraints

- Active original checkout/branch only; generated work under AROS/local. No additional active checkout.
- x86_64 ABIv11 only; v1/arm64/Macaros deferred. No cross-ABI binaries.
- Existing shared SDK/sysroots/guest libraries remain unchanged; new std changes use a new private installation.
- Clean upstream/registry/other projects are read-only; changes are owned versioned patches.
- No old v11 OpenSSL1 backend. Mbed TLS chosen here; do not claim rustls support.
- Normal English commits without attribution trailers; no push.
- Guard shared VM ownership, stage stopped, collect stopped, release; bound every test.

## Review focus

- Descriptor namespaces: a DOS file/pipe must not be inserted into socket WaitSelect.
- Task ownership: cross-task operation must be verified or explicitly refused; no ignored errors.
- Readiness races: no lost notification between registering interest and waiting; preserve oneshot rearm.
- Cleanup: finite timeouts, EOF, connection refusal, cancelled future and repeated creation/drop leave no owned workers/sockets.
- TLS failures: required chain/hostname/date validation, entropy-source failure and cleanup after unsuccessful handshake.

### Task 1: Pin inputs and establish socket readiness boundary

Files: docs evidence/current-state; std-groundwork/libc-aros/{src/lib.rs,csrc/*}; new std-groundwork/async-network acceptance/fixture tools.
- [x] Record exact Zed revision/lock hash and published crate checksums; audit apkg sources and ABI-specific recipes.
- [x] Run bounded native socket probes against a private host fixture: nonblocking, pending/readable/writable/EOF, deadlines, close/reuse, and socket owner task.
- [x] Before each fix, observe a failing behavioral check; implement minimal socket C/Rust boundary changes, preserving existing file/process semantics.
- [x] Host/unit regression checks and named v11 runtime evidence; commit the independently reviewable boundary.

### Task 2: Real pinned async TCP consumer

Files: ports/async-network prepare/build + versioned dependency patches; acceptance package/tests.
- [x] Compile pinned consumer to obtain actual API/feature gaps. Do not infer support from target cfg alone.
- [x] Add an AROS selector using verified socket readiness and task-signal wakeups, with level/oneshot semantics and explicit edge/foreign-task errors if unsupported.
- [x] Adapt async-io nonblocking/connect/driver ownership based on Task1 measurements; keep any owner-task restriction visible in APIs/docs.
- [x] Run delayed host reply, timer race, cross-thread wake notification, concurrent logical tasks, cancellation/drop, refused connect, EOF and repeated lifecycle cases.
- [x] Prove the same cases fail without required adaptation and pass after it; retain output/provenance and commit.

### Task 3: Stepped Mbed TLS HTTPS on the accepted async boundary

Files: ports/async-network TLS C adapter/build recipe, Rust wrapper and controlled TLS fixture.
- [x] Copy and hash apkg Mbed TLS3.6.7/jitterentropy3.7.0 sources/config/port glue; preserve upstream compile flags (jent -O0/-fwrapv).
- [x] Opaque context with handshake/read/write steps; distinguish WANT_READ/WANT_WRITE/error, explicit resource ownership, required CA/name/date checks.
- [x] Bounded local certificate fixture: trusted valid exchange; wrong hostname; expired certificate; untrusted CA; forced entropy failure. No host trust-store/guest settings changes.
- [x] Native checked results plus lifecycle after rejected connections; commit only demonstrated scope.

### Task 4: Repeatable acceptance and integration review

- [ ] Common preparation/build/fixture/report recipe with pinned inputs, immutable build-owned copies, unique run IDs and stale-result guards.
- [ ] Rerun relevant existing libc/process/LSP regressions after boundary changes; retained independent entropy contract.
- [ ] Fresh read-only review, fix material findings with meaningful tests, final native verification and ordinary commits.
- [ ] Update accepted/backlog state; distinguish source/build/link/runtime, natural-load limits and remaining full-Zed requirements.

## Execution rulings

- Implement inline in the existing branch; two independent read-only audits are parallelized under dispatching-parallel-agents.
- The selected named profile remains the older AROS One v11 pool profile. A newer base is not substituted silently.
- Task ownership is an investigation gate, not an assumed platform limitation. Owner-task-only async acceptance is allowed if evidence requires it, with unsupported executor modes left open.
