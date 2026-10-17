# Isolated older v11 runtime regressions

Three probes compare pthread detach lifecycle, parking_lot realtime deadlines,
and explicit Rayon global shutdown. Host `observe.py` waits at most60s per case
for start and actual Shell-return receipts; missing end never passes. Baseline
exit hangs must be stopped by the owner through the VM pool, not guest `_Exit`
from a worker. Receipts are run-specific test evidence, not authenticated remote
attestation. Retain native stdout, Shell rc, host observations and build manifest.

The detach probe reads the pinned pthread internal state under Forbid solely to
order completion/slot reuse deterministically. It never modifies that state.
The timedwait wrapper logs/checks deadline against realtime and calls the real
SDK function unconditionally. Wakeup can race before kernel timedwait; only the
unnotified timeout requires a timedwait call. Rayon checks private/global worker
exit handlers, but the subsequent Shell helper is the proof of process exit.

Build with a private JSON config naming `toolchain`, `cargo_home`, `gcc`, `sdk`,
`ar` and read-only `aros_src`. The archive cache must contain the pinned crates.

```
python3 ports/runtime/build.py CONFIG NEW_BUILD
```

Recipe first compares all33 plain SDK objects, then compiles two versions of the
detach test; Rust comparisons both prelink the fixed pthread objects. Only the
`after` Rayon build enables feature `shutdown`, calling the added API. The Rayon
API is opt-in and target-independent; it is not installed into shared crates.

Use VM pool status/reserve, stopped-only stage of **NEW_BUILD/payload** (the
controller creates POOLINPUT:payload.zip), then boot and extract to RAM:rt.
Do not wrap payload in another ZIP. Start the observer after extraction:

```
python3 ports/runtime/observe.py NEW_BUILD/build.json before NEW_OBSERVATIONS/before.json
```

Check its ready record, then execute `RAM:rt/run-before` in the guest. Cases run
in independent Shell tasks so all three bounded failures can be observed.
After observations finish, stop and collect the owned VM. Reboot with the same
immutable payload, extract to fresh RAM, observe variant `after` into a fresh
file and Execute `RAM:rt/run-after`. Collect while stopped and release.
No reports are overwritten; each case preflights its unique log and rc.
The initial start window is60s and each started program gets60s separately.
The observer exits finitely but does not kill processes: stop only the owned VM
after a timeout, before further staging. Never treat `Run` returning as completion.

```
python3 ports/runtime/check.py NEW_BUILD BEFORE_RESULTS AFTER_RESULTS NEW_OBSERVATIONS NEW_VALIDATION.json
```

This requires matching receipts, successful actual exits after correction,
expected baseline diagnostics, retained source/object/payload digests and pool
collection hashes. A failed setup or unexpected baseline cannot be accepted.

Accepted native before/after run and limits:
[2026-10-17 evidence](../../docs/evidence/2026-10-17-runtime/REPORT.md).
Configuration compiler/SDK paths must match the selected aros-env.toml and target
template; this recipe does not provision a different toolchain/ABI.
End receipts read the actual freshly written rc file, avoiding Shell variable
expansion assumptions. Existing patch headers retain their original verification
statements; the linked report records the new isolated native tests.
