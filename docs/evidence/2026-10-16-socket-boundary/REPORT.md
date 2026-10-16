# Scoped socket boundary acceptance, 2026-10-16

Accepted only on x86_64 ABIv11, older AROS One pool v11-2, GCC10.5,
AROS/sdk, private Rust alias aros-net-20261016 (1.100.0-nightly 0dfb098f3).
No new v1, arm64/Macaros, async-net, full smol, TLS or Zed runtime acceptance.

## Reproduced failure and correction

The unchanged std socket probe connected to the private host fixture with BSD
fd 0. set_nonblocking(true) returned Ok, but the next read blocked rather than
returning WouldBlock. The host saw a connection and no application request.
Ending that fixture released the blocked read with EOF and the assertion failed.
The retained first stdout log ends at NONBLOCK Ok; screenshots and the host
session additionally captured the unexpected-EOF panic. This is the RED run.

The raw BSD number collided with DOS stdin. std socket operations also use
ordinary fd close/write/vectored/fcntl/ioctl paths, so fixing only readiness
would leave descriptor lifetime and writes wrong. libc-aros now gives sockets
private tagged handles and routes the relevant Rust libc operations to BSD
socket or ordinary DOS calls. WaitSelect receives validated decoded BSD indices,
never the high tags. The first corrected run returned WouldBlock, became readable
at 253ms after the fixture's delayed response, and preserved stdin after drop.

The new descriptor representation is a process-local libc-aros contract.
Unadapted C consumers must not use AsRawFd values as raw BSD descriptors.

## Final native gate

`rust-net-06.log` is the final complete acceptance run. `fixture.jsonl` records
all 83 controlled connections and an error-free completion. Assertions cover:

- nonblocking pending read and connect-timeout path;
- readable/writable readiness, finite deadline, delayed data and EOF;
- vectored output, closed/oversized descriptor rejection and unsupported events;
- 80 sequential socket open/connect/read/drop cycles and stdin preservation;
- ordinary DOS file write/read/dup/shared offset/drop;
- no-argument FIOCLEX;
- explicit foreign-task DNS and socket I/O rejection;
- foreign-task drop followed by owner-side cleanup and another connection.

The adapter deliberately restricts socket I/O to the opening task. This is a
measured adapter contract, not a general claim about all AROS networking APIs.
Foreign drops remain queued until guarded owner I/O/creation/readiness or library shutdown;
there is no idle-owner wakeup yet. Socket duplication is ENOTSUP. Poll supports
nonzero IN/OUT requests; generic files/pipes, urgent data and HUP/ERR classification
are not accepted. See the acceptance package README for the exact contract.

## Independent review and regressions

Read-only review identified four issues before acceptance: failed deferred closes
lost ownership; resolver calls bypassed owner checks; ioctl consumed nonexistent
arguments; select exceptions were mislabeled POLLERR. All four were corrected.
The production deferred-close function is compiled with a failing CloseSocket
boundary in the host regression: failed ownership remains queued and the next
call retries. Its missing error-result contract failed before the fix, then passed.
The full host suite passes 34 tests.

Freshly rebuilt native regressions use the same compatibility source hashes as
the socket probe and selected private std. Entropy/getrandom/rand and four joined
HashMap threads pass. Five process cases pass: status, 128KiB stdout/stderr,
arguments, spawn-error recovery, bytewise LSP framing. The existing seven-case
unwind LSP baseline passes its manifest/receipt validator: baseline, interactive,
shutdown, active-exit, active-EOF, unwind and cancellation. `lsp-validation.json`
retains those case/report hashes. This repeats that previously scoped gate;
it does not expand its natural-load/backpressure claims.

## Incomplete runs and newly exposed gap

Run03 completed socket readiness and 80 lifecycles, then the fixture's Unix
/tmp filename failed (ENOENT). The test now uses explicit RAM:. Run04 reached
File::try_clone and returned EINVAL. Source inspection and the explicit final
assertion identify the profile's declared-but-unimplemented F_DUPFD_CLOEXEC.
The ordinary libc::dup regression is independent and passes; File::try_clone
remains open. Do not report it as supported based on this gate.

Run05 was not accepted: initial Shell execution could not open the script, and
by retry the host fixture deadline had expired. Its result file was accidentally
reopened by that retry and is excluded from the evidence set. Run06 used a new
result filename, a fresh finite fixture and direct binary execution. Only its
final PASS plus the 83-connection completion establish final acceptance.

## Provenance, environment and cleanup

`acceptance.json` records source, binary and report hashes. Every selected guest
report was checked against the controller's read-only stopped collection manifest.
Full local build/probe logs and stopped collections remain in
AROS/local/rust-async; LSP build/run metadata is in lsp-regression-01.
The original accepted aros-exec toolchain and shared SDK were not modified.
The new private toolchain is a copy whose wrappers select its own sysroot.

All payloads were staged stopped, extracted to RAM:, and reports collected stopped.
The final gate leaves no socket fixture workers; v11-2 was stopped and released.
No persistent guest setting or system library was changed. RAM-only test files
vanish at power-off; unique RESULTS files remain. Other owners' VMs were untouched.
