# Accepted delivery gate: repeatable x86_64 v11 LSP baseline

Accepted 2026-10-16 on the named older pool v11-2 / GCC10.5 / AROS/sdk /
aros-exec-20261015 profile. This closes the scoped M3 process/LSP delivery gate,
not M3 async networking or a Zed port.

## Closing criteria and evidence

1. Active-exit and active-EOF worker cancellation/completion/join: accepted
   with response accounting, exact trace order and zero return codes.
2. Explicit abort/unwind build selection: shared helper, abort default,
   isolated output and rejection of ambient overrides. Compiler cfg verified;
   fresh native abort baseline and full unwind bundle both pass.
3. Common entry point: baseline.py build prepares/builds/stages and records
   inputs/logs; baseline.py check validates stopped guest reports. VM ownership,
   focus inspection and collection remain the shared testbench workflow.
4. Regressions and limits: native LSP/probes, 33 Python tests, guarded unique
   output prefixes, start/completion receipts and failure-status propagation.
   Existing entropy/transport/process adapters remain unchanged.

See [scoped evidence](../evidence/2026-10-16-wat-baseline/REPORT.md) and
[commands](../../ports/wat-server/README.md).

## Remaining work

M3 async readiness/network/HTTPS and representative Zed/GPUI dependencies
follow separately. Natural-load interruption, blocked writer/noncooperative
worker termination, other SDK profiles, v1 and arm64/Macaros remain open.
No full-Zed delivery date, hermetic SDK distribution or bit-identical build
claim is made by this gate.
