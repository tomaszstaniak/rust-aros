# Isolated runtime before/after acceptance — 2026-10-17

Accepted run `e41ec53f-413d-4b91-9818-92f453b0e24c` (private bundle02), older
x86_64 ABIv11 / AROS One1.3 / pool v11-2 / QEMU TCG. Rust toolchain
aros-net-20261016 reports rustc1.100.0-nightly (0dfb098f3 2026-08-31); Rust
programs use explicit unwind/build-std from owned libc copies. C uses
x86_64-aros-gcc10.5.0 and the matching older v11 SDK. Configuration paths were
checked against aros-env.toml; SDK pthread archive digest stayed unchanged.
Reuse the same-day platform review in ../2026-10-17-project-audit/REPORT.md;
no mainline changes were installed or assumed to apply to v11.

## Native comparison

All six binaries were built together before running. Both variants use the
same VM profile, toolchains, SDK, program bodies and dependency versions. Only
detach's pthread objects differ. Parking/Rayon both prelink corrected pthread,
isolating their crate changes. Rayon before omits the unavailable shutdown API;
after explicitly enables feature shutdown and calls the new API. This necessary
call-site difference is recorded in build.json, not hidden as identical source
behavior. Both use the same two-worker private/global workloads and checksums.

| Probe | Before | After |
|---|---|---|
| pthread detach | 64 detach-before cycles reuse slot1; first detach-after-finish leaves slot occupied; exit hangs to60s host limit | 64 cycles each ordering reuse slot1, rc0 and actual Shell exit in8.068s host |
| parking_lot0.12.5/core0.9.12 | Actual timedwait deadline differs from realtime by-1792235835s; no completion within60s | Realtime deadline diagnostic, 300ms timeout and eight predicate wakeups with joined workers; rc0/actual exit in1.978s host |
| Rayon1.12/core1.13 | Private pool's two workers exit; global work finishes and main reaches return, process still hangs to60s | Private/global worker exit counts2 each; terminate-before-init no-op and repeated terminate tested; rc0/actual exit in0.990s host |

The requested300ms timeout measured405360us on guest Instant in this run.
This is a bounded behavior check, not a timing/performance benchmark. Guest/host elapsed
values are not compared to each other. Each started program has a60s host
budget, plus a separate60s setup/start window. Timeout is expected RED here,
never a successful native outcome. Baseline exits have no fabricated rc files.

The detach C diagnostic reads pinned ThreadInfo finished/task state under
Forbid to order tests; it never edits slots. The source commit is
5376f09bc1ed0c36a12fbb096406013f8a046e89 (deadwood2/AROS compiler/pthread).
All33 unmodified SDK members match rebuilt instructions/relocations before the
two-object private patch. Raw rebuilt object digests are retained in build.json.
The64-cycle test establishes reuse, not a test of all2019 concurrent slots or
all detach/join/cancellation races.

Parking's link wrapper logs/checks the actual absolute deadline against
CLOCK_REALTIME and always calls real pthread_cond_timedwait. It does not replace
its behavior. Wakeups may occur before the kernel wait, so only the unnotified
timeout requires an observed timedwait call. No system clock is changed. This
does not add CLOCK_MONOTONIC condattr support; core chooses realtime explicitly.

The independent Shell helper reads the freshly written Echo$RC file, strictly
parses it and reports end only after the probe process has returned. Rayon
worker-exit markers alone are insufficient. The global shutdown API remains
opt-in; no work may be submitted to the global pool after shutdown. Concurrent
shutdown/submission and other OS targets are not accepted by this probe.

## Validation and provenance

ports/runtime/build.py verifies checksum-pinned crate archives, compares the SDK
objects, builds plain/patched copies outside the repository and writes a
built-not-run manifest. Runtime acceptance lives separately in validation.json.
ports/runtime/check.py verified retained inputs, staged payload digests, nine
collected files against pool SHA manifests, exact native identities/markers,
expected baseline hangs/clock mismatch and three matching successful bounded
Shell-exit receipts. Full local outputs/collection manifests remain at
AROS/local/rust-runtime-20261017-02. No binaries, SDK objects or external source
trees are committed; build.json input hashes reference those retained files.
All three observer cases passed with errors[]. VM stopped/collected/released;
no active lease or fixture remains owned by this run.

Patches snapshot the public Photocraft port's existing pthread/parking/Rayon
adaptations, preserving headers/origin. Their historical verification paragraphs
refer to that snapshot; this report adds isolated native acceptance. Shared SDK,
sysroots, upstreams, Cargo registry sources and Photocraft were not modified.
The existing private Cargo cache may receive normal Cargo metadata writes.

Host suite66/66 passed. Independent read-only review found baseline validation
could accept generic failure: two behavioral negative tests first failed, then
passed after requiring actual timeouts and a measured clock-domain mismatch;
full suite passed. No deferred review findings. Additional strict exit-code
parser tests execute the real C parser on valid/invalid/missing files.

## Excluded attempt

Bundle01 (`2494061e-fce9-4a92-99ab-dbe3843651bb`) reproduced the same baseline
failures, but its after scripts wrote literal $probe_rc and the observer rejected
all three end codes. Native transcripts alone were not accepted. Full checker
also rejects it (rejected-01-check.log); no validation file was written. The fix
uses Echo$RC directly and a file-reading helper. Fresh bundle02 repeats the
entire comparison with new report names. An initial listener bind denied by the
host sandbox was corrected before any probe received a valid start receipt.
Setup also briefly double-wrapped the pool's automatically created ZIP; no test
was accepted from setup attempts. They are not mixed into this accepted run.

## Scope and decisions

This is an isolated extension of runtime tests, not a Photocraft rebuild or a
change to the installed SDK. It does not accept Fontique/GUI, Tokio networking,
socket Send, v1 or arm64. Existing mmap/combined networking results retain their
own scope. The bounded host observer does not attempt unsafe worker-side _Exit;
on a baseline timeout the owner stops its VM, at the cost of rebooting between
before and after. Reproduction needs the same compatible configuration paths
as the repository's selected aros-env.toml and target template.
