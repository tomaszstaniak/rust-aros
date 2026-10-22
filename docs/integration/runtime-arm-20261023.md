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

## Private detach correction and x86 control

After the native baseline failed, the existing versioned pthread.patch was
applied privately to pthread.c and pthread_create.c only. Both 64-cycle phases
then passed on rpi-1 and the Shell wrote RC=0 plus the completion receipt.
No shared SDK archive or running installation was replaced.

The runtime compilation source is pinned separately to
cc39c62f33d90b842540e79d5e47357590c014ac. The newer diagnostic-header checkout
contains a Macaros per-task errno hook absent from the native SDK: attempting
to rebuild that runtime correctly failed to link __stdc_errnoptr_hook. No fake
hook or errno replacement was added. The older source has the required data
layout and no such dependency. Its recompiled instructions are not claimed to
match the SDK object-for-object; manifests identify the source, private object
hashes and actual SDK separately. --fixed is opt-in in build-arm-detach.py.

On separately reserved v11-2, the previously accepted x86 detach/parking/Rayon
control binaries were checked against their original build manifest before
staging, then rerun under fresh rtxc23 receipts: all three returned RC=0, with a
final completion receipt. These are controls of the existing accepted v11
runtime (including its opt-in fixes), not a claim about an unmodified old SDK.
The busy v11-1 remained untouched. Evidence: detach-fixed-results and
x86-control-collected in the private session root.

## Parking and Rayon unmodified-crate baselines

After correcting the reproduced detach dependency, the shared guarded Rust
compiler built the existing Cargo probes with panic=abort and no parking/Rayon
patches. Rust 0dfb098f3 and the native SDK are unchanged. Closed unique Rayon
checkpoint files are diagnostics, explicitly not exit receipts.

Native ARM parking_lot 0.12.5/core 0.9.12 selected its generic backend (the target
has no unix family). The 300 ms timeout measured 300841 us; all eight predicate
wakeups and joins completed; the expected zero pthread timedwait count was
checked for this backend. Shell RC=0. No parking patch was applied. The test
retains the previous mandatory timedwait check when no generic backend is
explicitly declared, preserving the old x86 contract.

Unmodified Rayon 1.12.0/core 1.13.0 completed private pool shutdown (two worker
exit handlers), global computation and the before-return checkpoint. It did
not return to the Shell: no Rayon RC or final completion receipt before the
60-second deadline. This reproduces global-worker/process-exit failure on ARM,
rather than merely inferring it from x86 or an apparent PASS message. Evidence:
parking-rayon-baseline-results/RTPR23, including the three closed checkpoints.

The new build-common.py reuses the guarded compiler/profile, verifies SDK and
C bridge hashes, records consumer/lock/object hashes and selected backend, and
never patches crates by default. Rayon --shutdown is an explicit private variant
using the already versioned patch, reserved for after this baseline failure.
All 98 host Python tests passed; the unpatched host Rayon control returned 0.

## Final bounded ARM acceptance

The existing rayon.patch was enabled only after the baseline above reproduced
failure. The private build used --shutdown, with the already verified detach
objects. The test calls shutdown before registry creation, then twice after
work, and requires two global worker exit handlers before returning.

Final rpi-1 native AROS/QEMU execution, in detach -> parking -> Rayon order,
produced RC=0 for every case and a fresh final completion receipt:

- detach: 64 cycles before finish, 64 after finish, same slot reused;
- unpatched parking_lot generic backend: timeout 307151 us, eight wakeups;
- Rayon with opt-in shutdown: private exits=2, global exits=2, process returned.

Every run had an absolute 60-second host observation deadline; the final owned
VM stop completed after 60.362 seconds. Checkpoints or before-return output were
never substituted for actual Shell receipts. No ARM unwind or sockets were
added. The new verifier also rejects another CPU/SDK, stale patches, missing
object inventory and changed private pthread objects before starting a build.
All 105 host Python tests passed. Independent read-only review found no P1/P2.

Reproduction uses a guarded common sysroot and retained normal-Cargo build
metadata as profile input. Build detach baseline and corrected variants with
ports/runtime/build-arm-detach.py (only the latter passes --fixed), then:

```
python3 ports/runtime/build-common.py PROFILE_BUILD COMMON_SYSROOT NEW_PARKING --case parking --pthread-objects FIXED_DETACH_BUILD
python3 ports/runtime/build-common.py PROFILE_BUILD COMMON_SYSROOT NEW_RAYON_BASELINE --case rayon --pthread-objects FIXED_DETACH_BUILD --progress-prefix SYS:PRIVATE_BASELINE/rayon
python3 ports/runtime/build-common.py PROFILE_BUILD COMMON_SYSROOT NEW_RAYON_FIXED --case rayon --pthread-objects FIXED_DETACH_BUILD --shutdown --progress-prefix SYS:PRIVATE_FIXED/rayon
```

Use the pool's reservation and stopped-only staging rules. Set a 4 MiB stack,
redirect to fresh result files, write Echo $RC immediately after each program,
and require completion before accepting the run. Progress prefixes name existing
private guest drawers; each phase writes a distinct closed file. Physical Pi
remains a separate gate. Native socket owner context on both CPUs is subsequent
work, not part of this acceptance.
