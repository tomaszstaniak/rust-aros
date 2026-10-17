# Dedicated SocketBase task: SDK feasibility — 2026-10-18

Outcome: positive SDK-level feasibility on older x86_64 ABIv11 / AROS One1.3 /
pool v11-2 / QEMU TCG. Run5670ae8b-db9d-4b2e-ae70-fed6a849562d, GCC10.5.0,
matching older v11 SDK and private corrected pthread objects from accepted
runtime bundle02. C-only probe: no Rust panic profile or worker std/async
acceptance is claimed. The same-day selected SDK pthread archive SHA still
matches the prior build; private objects and receipt helper were checked
against their accepted manifest. The platform-source review from2026-10-17 was
reused within its weekly interval; no new upstream update was installed.

Sixteen sequential pthread workers, each joined by main, each perform four
independent OpenLibrary/CloseLibrary cycles. Every worker verifies it is not the
main task. Socket calls use explicit-base SDK __*_WB definitions; no process-global
SocketBase is assigned by the probe. Each instance configures its private errno
pointer, creates listener and client, connects through127.0.0.1 to an ephemeral
port, accepts, sends seven known bytes, receives in requests of at most two
bytes, verifies payload, shuts down the write side, verifies EOF and explicitly
closes all three sockets on that owner task before closing the library.

All64 library lifecycles/TCP exchanges/EOF cases and16 joins passed. The separate
Shell receipt helper reported actual process return0 in3.361s host, within the
60s absolute host limit. Collected rc is0; both native reports match the stopped
pool collection SHA manifest. Input/payload digests and compiled run identity
were checked against build.json. The host observer is the hardened7b00bb2 tool;
its SHA is included in observation.json. Owned VM stopped, collected and released;
observer reaped. Nothing changed in shared SDK, Photocraft or Rust adapter code.

Private throwaway prototype/source/build/collection are retained at
AROS/local/socket-owner-20261018-02, identified by manifest hashes. The prototype
is not maintained product code and is not adopted into ports/. These retained
reports are evidence of the feasibility answer, not a new reusable transport.
Initial build01 failed on a missing socketbasetags header and did not run; fresh
build02 includes the correct SDK header. No failed attempt is accepted here.

## What this resolves

A task created through this pthread runtime can open/configure its own BSD
library instance, perform real TCP work and close it before joining/exiting.
A dedicated owner is therefore viable at the SDK boundary on this named guest.
This is stronger than testing socket allocation alone, but remains a small
bounded feasibility check, not a memory-leak/load benchmark or a complete
proof of resource exhaustion behavior.

## What remains unaccepted

The current Rust libc adapter opens a global main-task base and rejects foreign
I/O. It was deliberately bypassed here. Existing Rust async/TLS objects do not
become usable on this worker by virtue of this result. No proxy, Send+Sync
connector, cross-thread wakeup protocol, queue/memory budget, cancellation or
multi-client server was implemented or accepted. Nonblocking flow control and
forced partial writes were not tested: this prototype uses blocking loopback
with a host process deadline. Main/worker networking coexistence and failure
cleanup still need the production context tests.

Recommended next implementation boundary is the explicit owner context proposed
in ../../superpowers/specs/2026-10-18-socket-owner-lifecycle-design.md. Avoid
reassigning the global base underneath legacy sockets/selectors/TLS. Keep native
handles !Send; only later introduce bounded request/result proxies. The proposed
stage1 written design awaits review before production interface changes.
