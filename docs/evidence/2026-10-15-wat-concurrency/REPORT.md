# Outstanding LSP requests and cancellation boundary, 2026-10-15

## Scope

Active implementation: x86_64 ABIv11 only. v1 and arm64/Macaros are deferred
by the user's 2026-10-15 scope update.

Runtime: older AROS One 1.3 UP, pool v11-1, QEMU pc/TCG qemu64, two configured
vCPUs, 2 GiB. GCC 10.5 / AROS/sdk, isolated aros-exec-20261015 with Rust
1.100.0-nightly (0dfb098f3 2026-08-31). Same pinned wat_server 0.11.0 commit
428f74da7fcb85d2fca9bd9828f5ff7434c19af8, transport, libc and constructor
patches as [the previous acceptance](../2026-10-15-wat-lsp/REPORT.md).

No shared toolchain, SDK, registry source or guest runtime library changed.
Payload ran from RAM:wl, reports from RESULTS:. The owned guest was stopped
before collection and released afterwards. Exact payload hashes are recorded
in payload-sha256.json; full collection and manifest remain in
AROS/local/rust-lsp/results-03.

## Real LSP workload

After the existing diagnostics, repair, formatting and symbol checks, the client
sends 16 requests (IDs 10–25), alternating formatting and document symbols,
before reading their replies. Results are matched by ID and checked for actual
formatting edits or the expected function symbol. No response-arrival ordering
is assumed. The guest is UP: this establishes outstanding requests and threaded
service behavior, not simultaneous execution on multiple CPUs.

Native result: client rc 0, server exit 0, stderr 0 bytes, stdout EOF and joined
stderr drain. Raw evidence: wl03-transcript.txt and wl03-rc.txt.
The original upstream and patched host servers both pass the same client:
wat-batch-reference.log and wat-batch-host.log.

The new collector was tested red → green. Tests cover reversed reply order,
duplicate/unrequested IDs, error replies and missing results. Invalid fixtures
supply complete batches and assert the protocol rejection, not incidental EOF.
Both tests pass. The existing seven transport tests, five framing tests,
12-case process integration and all 23 Python tests pass; logs are retained.

## Deterministic engine cancellation

The separate cancellation-probe.rs runs the actual pinned LanguageService:
opens a valid WAT document, confirms ordinary symbol analysis, clones the
service, sets that snapshot's cancellation token, verifies it is set, and
requests document symbols. There is no timing race and no fabricated reply.

Host with unwinding: normal query passes, cancellation is caught, result None,
process exits 0. See cancel-host.log.

Native panic=abort: normal query passes, CANCELLATION_REQUESTED is written,
then the process aborts. The guest's Caught Signal requester named
RAM:wl/cancel-probe; it was dismissed so the script could record actual rc 20
and its final DONE marker. See wl03-cancellation.txt,
wl03-cancellation.rc and wl03-done.txt. This is a reproduced failure, not a
passing cancellation test.

Salsa 0.28.1 src/cancelled.rs implements throw with panic::resume_unwind and
catch with panic::catch_unwind. LanguageService::with_db uses Cancelled::catch.
Our target JSON selects panic-strategy=abort. Source and native behavior agree:
the current runtime cannot fulfill Salsa's unwind-based cancellation contract.

This does not test the full LSP $/cancelRequest delivery/response lifecycle,
forced process termination or a general unwind implementation. Live document
updates can also cancel Salsa queries; their overlap remains unaccepted.

## Reproduction and next work

ports/wat-server/build.py --cancellation-probe builds wat-cancellation-probe in
a disposable prepared workspace, using the existing dependency pins/patches.
stage.py accepts --cancellation-probe PATH and writes a separate probe report
and rc after the ordinary LSP session. See the port README.

Keep x86_64 v11 as the implementation target. Next: resolve the engine's
abort/unwind boundary with a scoped design and runtime test, then exercise
document changes overlapping requests. Do not paper over this result by
treating a sent cancellation notification as accepted cancellation.
