# ARM runtime regression gates, 2026-10-23

The requested local integration fast-forwarded codex/rust-renewal-m0 from
e8e267d to b263211, including the focused review. The original checkout's
modified .DS_Store retained SHA-256
b431b0addc5a8b9052afbd5846089f07f349ae73b890c145513e0cb21615612b.
No push or merge to main occurred. All 95 Python host tests and the Rust
acceptance workload passed in the merged checkout. Further work is isolated on
codex/rust-arm-runtime-regressions-20261023.

The existing TLS limitation is unchanged: first global pthread-key creation
inside a TLS destructor is unverified and can contend with AROS's shared TLS
semaphore. Creating another value under a pre-created key was tested separately.

## Detach baseline

Unmodified native Pi SDK 20260822, AArch64 ABIv1, AROS Clang 20.1.0, native
AROS on rpi-1/QEMU raspi3b. Existing ports/runtime/detach.c was reused; diagnostic
header 7c24ef2ef03cd0ef1cbf2bcbad4227f7d62d2652 was compiled against the actual
SDK. Static assertions require ThreadInfo stride 912, finished offset 32, task
40 and detached 900. SDK disassembly independently exposes stride 0x390,
task 0x28 and detached 0x384. No internal state is modified by the probe.

Observed: 64 detach-before-finish cycles passed; detach-after-finish cycle 0
reported `FAIL leaked-slot phase=1 cycle=0`. No shell RC or completion receipt
was produced before the 60-second host deadline. This is a reproduced baseline
failure, not acceptance. Both the layout assertion and the existing probe are
reproducible with ports/runtime/build-arm-detach.py; it never edits the SDK.

Private evidence: local/rust-runtime-arm-20261023/
contains merged-host-tests.log, merged-rust-test.log, detach-baseline-results,
detach-baseline-deadline.json, pthread-sdk-disassembly.txt and the private
source/build snapshots. The original 2026-10-17 x86 tests and patches remain
unchanged. The occupied v11-1 was not touched.

Next: private detach correction after this reproduction; parking_lot baseline
with its actual generic ARM backend; then unpatched Rayon process exit. Apply
runtime changes only following a reproduced failure, retaining separate
baseline/after evidence and explicit x86 controls.
