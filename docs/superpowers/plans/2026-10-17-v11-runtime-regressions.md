# V11 runtime regressions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Isolate and measure three existing runtime fixes on older x86_64 v11.
**Architecture:** Three small native probes and a private before/after builder.
A separate synchronous receipt helper reports start and Shell-observed exit to
a finite host listener. Preserve fresh identities and stopped-only VM collection.
**Tech Stack:** GCC10.5/ABIv11 SDK, Rust nightly2026-08-31, unwind,
parking_lot0.12.5/core0.9.12, Rayon1.12/core1.13.
**Spec:** docs/superpowers/specs/2026-10-17-v11-runtime-regressions.md

## Global Constraints

Original repository/branch; all outputs private and new; shared SDK/toolchain
and Photocraft read-only. Reuse the 2026-10-17 platform audit. Baseline pthread
objects must match SDK instructions/relocations. Only pthread changes between
detach variants; both Rust cases use corrected pthread to isolate crate fixes.
Rayon baseline omits the unavailable shutdown API, explicitly selected by feature.

## Review Focus

- Missing/stale/out-of-order exit receipt cannot pass: real listener tests.
- Hanging exit handler must not pass on a pre-exit marker: baseline Rayon run.
- Detached slot reused while worker still executes: inspect under Forbid and
  wait for the runtime's finished/null state; no arbitrary sleep for ordering.
- Wrong clock can busy-spin or hang despite public timeout: wrap the real
  timedwait for deadline diagnostics, never replace its return behavior.
- Crate/source drift: locked offline sources/checksums and SDK object comparison.

### Task 1: Native probes and bounded observation

**Files:** Create `ports/runtime/{detach.c,receipt.c,observe.py,README.md}`,
`std-groundwork/runtime-regressions/{Cargo.toml,src/bin/parking.rs,src/bin/rayon.rs}`,
`tests/test_runtime_observation.py`.
**Interfaces:** Produces JSON observations keyed by run/variant/case and actual
Shell return code; native programs consume no application assets.
- [ ] Write real loopback tests: matching start/end pass, nonzero rc fails,
  missing end times out, stale identity/end-before-start fail.
- [ ] Run unittest test_runtime_observation; observe missing implementation RED.
- [ ] Implement finite receipt observer and three probes with explicit assertions.
- [ ] Run focused host tests GREEN and full unittest suite.

### Task 2: Private before/after recipe and native evidence

**Files:** Create `ports/runtime/{build.py,patches/*}`; modify lockfile,
`docs/evidence/2026-10-17-runtime/REPORT.md`, `docs/current-state.md`.
**Interfaces:** Consumes probes/observer from Task1; produces new build.json,
payload and observations; never claims built-not-run as runtime success.
- [ ] Build baseline before applying fixes; verify33 SDK object matches and
  checked crate archive digests. Record build/link failures honestly.
- [ ] Run baseline cases with60s per-case host deadline on owned older v11.
- [ ] Apply only named private fixes; build after binaries same unwind profile.
- [ ] Run after cases with same60s deadline and verify logs/rc/exit receipts
  against collected hashes, no stale report reuse.
- [ ] Run full host suite, fresh code review, fix material findings with tests.
- [ ] Retain evidence, update accepted limits, commit ordinary English changes.
