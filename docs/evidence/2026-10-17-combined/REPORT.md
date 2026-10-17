# Combined older x86_64 v11 acceptance — 2026-10-17

Fresh bundle05, run `e31d2474-1941-4b2c-9907-1347133b29db`, unwind profile,
private aros-net-20261016 toolchain, older v11-2. Strict checker passed after
stopped-only collection; all 42 required guest reports match collection hashes.
Owned VM stopped and released; all three fixture children reaped with exit0.

Accepted together: socket boundary83 connections (two intentional empty closes),
owner-task async consumer88 connections, entropy including threaded use, five
process cases, seven WAT LSP/unwind/cancellation cases, stepped Mbed TLS with
certificate negatives and forced jitterentropy failure, and exact
memmap2 0.9.11/read-fonts0.41.0 mapping/drop/font diagnostic. Component libc source
digests match. TLS fixture observed224 ClientHello bytes before cancellation;
host elapsed7.913s explains why the previous5s pending peer was premature.
Only stalled-peer budget is60s; ordinary TLS I/O remains5s, whole fixture300s.
Re-execution refused existing evidence with rc20. Host suite51/51 passed.

Attempts01–04 are excluded: missing explicit entropy build cfg; socket fixture
empty-close contract; invalid host-delay/guest-clock comparison; premature
pending-peer timeout. Fixes have focused host regressions. Timer completion now
opens a control connection that releases the waiting server response, proving
ordering without comparing host and guest clocks. Async connection count is88.
Fresh read-only review identified child accept deadlines shorter than the
controller; explicit300s deadline propagation and finite-deadline test fixed it.

Retained suite identifies immutable private inputs; local full build and pool
collection remain under AROS/local/rust-acceptance-20261017-05 and
AROS/local/rust-mmap-20261017/results-gate-05. No binaries or font fixture are
committed. Retained report/fixture hashes are in validation.json and SHA256SUMS.
The suite input hashes reference full local build inputs, not vendored files.

This accepts the scoped owner-task async-net backend, not Tokio networking,
general POSIX mmap, whole Photocraft font UI, arbitrary executors or Zed.
Next integration: rebuild Photocraft against the corrected libc privately and
exercise actual Fontique scan/cache and repeated GUI font use. Tokio bridge
needs a deliberate owner-task/Send-channel design plus listener lifecycle;
see ../2026-10-17-project-audit/REPORT.md. v1 and arm64 remain deferred.
