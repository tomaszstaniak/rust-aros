# Dedicated socket owner: lifecycle design

Status: proposed production design for user review. The separately recorded
SDK feasibility probe is throwaway, not an implementation of this design.

Native SDK feasibility passed on older v11-2:16 joined threads,64 private base
lifecycles and real loopback exchanges, process rc0. See
[report](../../evidence/2026-10-18-socket-owner-feasibility/REPORT.md). This
does not accept worker use of the existing Rust adapter.

## Intent and scope

Target older x86_64 ABIv11, with v1 and arm64/Macaros deferred. Build the first
layer for a dedicated socket task and later Send proxies, GUI bridge and async
consumers. Photocraft, shared SDK/sysroots and upstream checkouts remain
unchanged. Initial native feasibility is followed by an isolated lifecycle
implementation; bounded proxy queues/listener service/connector integration are
later gates, not claims of tokio::net support.

The user supplied the four-stage sequence and success conditions. This design
selects how stage1 fits the current Rust adapter. It does not authorize silently
promoting raw descriptors or current !Send async handles to Send.

## Current boundary

libc-aros/csrc/compat.c defines process-global SocketBase, socket slot table,
owner task and selector list. ADD2INIT opens the base on the main task;
rs_socket_ready rejects foreign I/O. ADD2EXIT closes that main-task base.
Selector resources are tied to its opening task. The current real async
backend uses these globals; moving only its block_on call to a Rust worker
would fail the owner guard and would not establish a new library instance.
Stepped TLS also retains its own existing owner restriction.

## Approaches

1. Keep the accepted main-task owner and add message proxies there. Smallest
   migration, useful fallback; consumes GUI/main-task scheduling and does not
   answer the requested dedicated-task design.
2. Close the main base and transfer the process-global role to one worker.
   Requires quiescence of every old socket/selector/TLS context and changes
   startup/exit assumptions. A pointer reassignment is insufficient. Rejected
   for the first implementation because legacy main-task users could regress.
3. Recommended: a separate explicit owner context. Use SDK base-specific
   calls with the context's SocketBase, not the process-global identifier.
   Its resources stay on its creator task and coexist with the legacy route.
   It costs a new small transport boundary and later driver integration;
   it avoids pretending ordinary std::net objects changed ownership.

## Stage1 ownership contract

An opaque native context records its creator task, private SocketBase and live
owned socket resources. Creation opens/configures the library on that task.
Each native entry validates the creator before touching mutable state or calling
BSD APIs. Wrong-task operations fail with ENOTSUP. Descriptor values remain
private to the context and never cross the proposed proxy boundary.

Rust OwnerContext and OwnedSocket are !Send and !Sync, with Rc ownership of
one inner context. Every socket retains that inner context. Dropping the public
context handle while a socket exists does not close its library; last-resource
cleanup occurs on the same creator task. Native explicit destroy refuses EBUSY
while socket resources remain, rather than unloading live socket state.
Socket close failure is reported/retains ownership, not disguised as success.

The service thread must drop sockets and its final context before returning;
the parent joins it before process exit. No detached service thread or main-task
exit hook closes a worker-owned base. Repeated startup creates a fresh context,
not a reusable handle to a retired task. Startup failure publishes no ready
owner and does not alter the legacy main-task base.

Stage1 needs TCP creation, nonblocking configuration, connect/readiness,
read/write/EOF and close sufficient for lifecycle tests. Listener/accept may
serve as a local diagnostic peer, but a multi-client server API is stage3.
The existing polling/async-io backend is not switched to the new context in
stage1. SDK-level feasibility alone cannot accept that backend on a worker.

## Native acceptance before stage2

- The dedicated thread opens/closes its own base and performs real TCP I/O;
  all socket calls, closures and library close happen on that opening task.
- Repeat at least16 joined owner threads and64 library lifecycles. Verify
  payload, deliberately bounded reads, EOF and explicit socket closes.
- Run legacy main-task socket/async controls before, during and after a worker
  context; its base/ownership must remain independent.
- Compile-negative test rejects moving safe owner/socket types to another
  thread. Direct foreign native calls reject before mutable/native I/O.
- Drop context handle with a live socket: socket remains usable; final cleanup
  follows last socket drop. Explicit native destroy with live resources refuses.
- Test failed startup and close failure ownership separately; a failed cleanup
  cannot permit an apparently clean restart or accept a passing exit marker.
- Use fresh run/binary identities, private source copies and SDK-matching pthread
  objects. A60s absolute host deadline bounds each program. Separate Shell exit
  receipts plus numeric rc and stopped-only collection prove actual process exit.
  A baseline timeout with any rc file/manifest entry is contradictory evidence.

The raw SDK probe only establishes the first two bullets' feasibility. It does
not satisfy the full production-context acceptance above.

## Constraints carried into stages2–4

Socket/driver handles stay !Send at the owner. Only bounded requests, bounded
owned data and completion/cancellation state cross threads. Admission limits
must cover queued, in-flight and unconsumed response bytes, not only message
count. Full queues reject before execution; stop/cancellation must remain
available without a free data-queue slot. No dropped client or abandoned result
may accumulate unbounded buffers.

Cancellation distinguishes not-started from started work. Losing a completion
after I/O started yields an unknown outcome unless the owner can prove the
result. There is no automatic replay, especially for future document mutations.
Partial writes expose only confirmed progress; application-level deduplication
would need its own explicit contract.

Wake registration, task retirement and notifier lifetime need a race-proof
protocol before a Send proxy is exposed; an atomic alive flag alone does not
prevent signaling a retired task. Owner shutdown must wake pending requests,
close active connections on the owner, retire notifiers safely and join. Later
native tests must stress lost-wakeup windows, bounded memory, several clients,
idle timeout, active-I/O shutdown and repeated start/stop, then satisfy the
actual connector Send+Sync/read-write contract in a separate consumer.

TLS, DNS, IPv6, tokio::net and GUI integration retain their previous limits;
the first context implementation does not silently extend their acceptance.

## Review checkpoint

This is a concrete stage1 design, not a completed implementation plan. Review
this ownership/lifecycle boundary before changing production interfaces. Once
approved, prepare the stage1 implementation plan and its native test sequence.
