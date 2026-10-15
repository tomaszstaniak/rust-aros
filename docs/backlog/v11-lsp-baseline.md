# Next delivery gate: reproducible x86_64 v11 LSP baseline

Updated 2026-10-15. This is a scoped M3 process/LSP gate, not completion of
M3 async networking or a Zed port. No calendar deadline has been agreed.
Planning estimate: two to three further complete implementation/validation
iterations, conditional on compatible VM availability and no new runtime issue.

## Closing criteria

1. Accept active-exit and active-EOF worker cancellation/completion/join on the
   named older ABIv11 profile; retain exact binaries, logs and return codes.
   Accepted on host and older v11-2 with explicit unwind; see teardown evidence.
2. Integrate explicit abort/unwind selection into the reproducible build flow,
   retaining abort by default and isolated outputs. Reject conflicting ambient
   flags and validate the selected profile rather than just a successful link.
3. Provide one documented entry point for preparation, build and validation
   reports, pinning source/toolchain inputs and preserving failure status.
4. Retain the current entropy, process framing, baseline LSP, cancellation,
   edit/recovery and teardown regressions. Clearly list unaccepted behavior.

## Scope and risks

Focus stays x86_64 ABIv11 with its named GCC/SDK/runtime combination. v1 and
arm64/Macaros remain deferred. Async readiness/network/HTTPS and representative
Zed/GPUI dependencies follow separately. No full-Zed delivery date is claimed.

The current constraint is turning individually verified paths into one
repeatable supported profile. Native VM contention delays the runtime gate;
a blocked writer or noncooperative worker may require a separate termination
policy and invalidate the estimate. Do not substitute a newer guest profile
or treat host success as native acceptance to meet the estimate.
