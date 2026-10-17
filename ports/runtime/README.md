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
