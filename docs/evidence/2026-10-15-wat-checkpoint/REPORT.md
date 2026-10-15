# Controlled Salsa checkpoint cancellation, 2026-10-15

## Context

x86_64 ABIv11, older AROS One 1.3 UP, pool v11-1 under QEMU pc/TCG
qemu64, two configured vCPUs, 2 GiB. GCC 10.5 and AROS/sdk, isolated
aros-exec-20261015 Rust 1.100.0-nightly (0dfb098f3 2026-08-31).
WAT 0.11.0 at 428f74da7fcb85d2fca9bd9828f5ff7434c19af8, existing
transport/libc/inventory patches, explicit private unwind profile.
The current platform review recorded in the earlier evidence remains applicable;
no SDK, system or compiler components changed for this probe.

## Probe design and limits

An optional versioned diagnostic patch installs a Salsa event callback and
arms only request 10 when --probe-checkpoint is passed. The first actual
WillCheckCancellation event on that worker sends a framed
$/arosProbeCheckpoint notification and pauses on a condition-variable predicate.
The client waits for that exact notification before sending $/cancelRequest.
The ordinary server preparation/build never applies this patch.

The server's existing notification handler finds and cancels the actual token,
records cancel_applied, then releases the diagnostic gate. Salsa resumes its
cancellation check and the existing catch boundary returns a null response.
No document change is sent between the checkpoint and that response: a pending
write is not an alternative cancellation cause in this controlled sequence.
The gate retains release-before-wait and a 20-second timeout visibly fails;
it is deliberately one-shot per server process. Client receive deadlines
remain diagnostic, with external supervision needed for blocked writes/waits.

This verifies wire cancellation at an active Salsa cancellation checkpoint.
It does not demonstrate interruption halfway through a tracked query body,
natural scheduling under editor load, latency, or production performance.
Synchronization and tracing alter scheduling. Earlier all-normal natural-load
runs remain valid and are not reclassified as interrupted requests.

## Results

Both host and native runs pass:

- Checkpoint notification precedes client cancellation; ID10 returns null.
- Trace order is registered -> worker_start -> salsa_checkpoint -> cancel_hit
  -> cancel_applied -> query_done -> write_ok -> token_removed.
- After cancellation, version100 edits produce empty diagnostics and a symbols
  query sees final_answer and v100_f1023, with no stale v1 names.
- A fresh server session acknowledges shutdown ID99 while ID10 remains paused.
  The client then explicitly cancels ID10 and receives its null response before
  sending exit. Trace confirms shutdown write_ok precedes ID10 cancel_applied.
- Both sessions reach EOF, child wait and both client reader joins, with
  client/server exit0. Stderr contains only the expected deferred trace.
- The separate baseline session passes its 16 outstanding mixed requests and
  ordinary shutdown, client/server0, stderr0.

No write_error, dropped trace or gate timeout occurs. Raw native reports are
in native-09/; payload-sha256.json identifies the executed binaries and script.
The full stopped collection and verified collection manifest are retained at
AROS/local/rust-lsp/results-09. All guest software ran from RAM:wl; only unique
wl09-* result files were written to RESULTS:. The guest was stopped before
collection and its reservation released. Shared SDKs, toolchains, sysroots,
upstream/registry sources and guest libraries/settings remain unchanged.

## Verification

Client tests: 9 pass (7 interactive, 2 baseline). Gate tests: 3 pass,
covering retained early release, no progress before release, and absent-release
timeout. Python tests: 23 pass. Transport: 7 pass. Process framing: 5 pass
plus the real-process integration containing 12 cases. Logs are adjacent.
Red-to-green was observed for the gate and checkpoint notification validation.
Host builds retain the existing unexpected target_os=aros cfg warnings;
native builds retain Cargo's build-std unused-patch warning. Builds complete.

## Reproduction

Prepare pinned sources with ports/wat-server/prepare.py --probe-checkpoint,
then build the disposable workspace with build.py --toolchain
aros-exec-20261015 --aros --unwind, using the documented private Cargo home
and target output. Build the acceptance package using its existing native
configuration and inventory patch. stage.py --interactive-client PATH
--probe-checkpoint stages baseline, controlled cancellation/recovery, and
outstanding-shutdown acknowledgement as three fresh server sessions.
The host client accepts SERVER --probe-checkpoint --trace-requests;
add --probe-shutdown for the second scenario.

## What this did not show and next step

Shutdown acknowledgement with an outstanding request is accepted only for
this synchronized sequence. Automatic cancellation on shutdown, rejection of
requests after shutdown, process exit with active workers, forced child kill,
ordinary-load active cancellation and generic buffered PIPE remain open.
The client explicitly awaits request10 before exit; no claim of joining
server worker threads is made. v1 and arm64/Macaros remain deferred.

Next: define and verify server teardown with active workers, then promote the
accepted explicit unwind profile into the reproducible toolchain/build flow.
