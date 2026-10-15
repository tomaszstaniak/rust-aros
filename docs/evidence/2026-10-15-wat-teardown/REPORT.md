# Request worker ownership and active teardown, 2026-10-15

## Scope and diagnosis

Pinned WAT 0.11.0 (428f74da7fcb85d2fca9bd9828f5ff7434c19af8),
Rust 1.100.0-nightly (0dfb098f3 2026-08-31), macOS arm64 host.
The existing diagnostic checkpoint pauses request10 at Salsa's actual
WillCheckCancellation event. A client receives shutdown99 while request10
is paused, then sends exit without explicit cancellation. Before the fix,
the server exits0 but no query_done/token_removed is recorded for request10;
the client rejects active-worker teardown with rc1 (host-before.log).
This is a host result about the pinned server, not an AROS runtime defect.

The pinned server drops every JoinHandle returned by thread::spawn. Returning
from its loop therefore does not establish completion of those workers.
The port-owned worker-lifecycle.patch now retains handles, reaps completed
workers before subsequent spawns, and on loop exit/EOF/error takes the token
map, releases its lock, cancels remaining tokens, then joins all retained
workers. Spawn failures and worker panics are errors. If the loop and cleanup
both fail, both errors are preserved through cleanup context.

The versioned lifecycle patch and workers.rs are applied by ordinary
preparation after transport.patch. Optional trace/checkpoint patches remain
separate. Diagnostic teardown releases its gate only after cancelling the
actual token, and records workers_joined only after successful finish.
No upstream checkout, shared toolchain/sysroot, SDK or guest system was changed.
No patch was sent upstream.

## Host acceptance

Final host logs show client/server0 for:

- Active exit: shutdown acknowledged with request10 paused, exit without a
  client cancel, teardown_cancel_applied10, query_done10, write_ok10,
  token_removed10, workers_joined, cancelled response accounted before EOF,
  child wait and both client reader joins.
- Active EOF: the same pending request, but stdin closes without an exit
  notification; cancellation, completion, join and response checks pass.
- Explicit wire cancellation followed by final-version recovery.
- Shutdown acknowledgement followed by explicit cancellation and response.
- Ordinary uninstrumented baseline, including 16 outstanding mixed requests,
  and ordinary interactive edits/recovery. The ordinary preparation/build
  contains lifecycle ownership but no checkpoint/trace code.

The diagnostic callback is synchronization at a cancellation checkpoint,
not natural editor scheduling or a mid-query-body interruption measurement.
The exit guard requires ordered checkpoint/query_done/token_removed/
workers_joined events, rejects truncation/write-error/timeout/panic markers,
and independently accounts for the cancelled response before EOF.

Tests: 10 client tests pass (8 interactive and 2 baseline), two worker tests
pass (join-before-return and panic propagation while draining other workers),
and all 23 Python tests pass. Seven transport tests and five framing tests
plus the real-process integration (12 cases) also pass. Red-to-green was observed for worker ownership
and the completion guard. Raw test/server logs are adjacent. Host builds
retain the known target_os=aros check-cfg warnings.

## Native acceptance

The final server and clients compile/link and run on x86_64 ABIv11,
older AROS One pool profile v11-2 under QEMU, explicit private unwind profile,
isolated aros-exec-20261015, GCC10.5 / AROS/sdk. This names v11-2 explicitly;
prior checkpoint results on v11-1 are not silently relabelled.
Known nonfatal Cargo build-std unused-inventory-patch and host
rust-objcopy/libLLVM warnings remain in builds.

Initially all ready v11 slots were occupied. When the compatible older v11-2
became free, it was explicitly reserved, staged while stopped and booted.
All five unique native report codes are 0: baseline, cancellation/recovery,
shutdown acknowledgement, active exit and active EOF. Client and server0;
baseline stderr0 and diagnostic sessions contain only expected deferred trace.

Both active teardown traces show:

    registered10 -> worker_start10 -> salsa_checkpoint10 ->
    shutdown99 response -> teardown_cancel_applied10 -> query_done10 ->
    write_ok10 -> token_removed10 -> workers_joined

The client sends no explicit cancellation in these two cases, and requires
the cancelled response before accepting EOF. No gate timeout, worker panic,
write error or dropped trace occurs. Cancellation/recovery additionally checks
the final-version document; baseline retains 16 outstanding mixed requests.
Raw native-10/ reports, payload-sha256.json and native-trace-analysis.json
identify the exact payload and per-ID ordering. Full stopped collection and
verified manifest: AROS/local/rust-lsp/results-10 (3914 collected files).

All software ran from RAM:wl; only fresh wl10-* report files were written to
RESULTS:. No installed libraries/settings or other owners' machines were
changed. The guest was stopped before collection and its reservation released.
No VM remains owned by this slice. The accepted scope is synchronized,
cooperative cancellation/teardown at the tested Salsa checkpoint, with output
continuously drained. The preceding host failure was the red control; no
pre-fix native failure is claimed.

## What this did not show

Joining does not impose a hard deadline. A noncooperative query or blocked
stdout can still block shutdown; forced kill/backpressure policy remains open.
Malformed input handling and post-shutdown request rejection are unchanged.
No general claim is made for arbitrary LSP servers, async networking, other
SDK profiles, v1 or arm64/Macaros. Combined loop/cleanup-error injection
is not claimed as tested here; the reviewed error match preserves both paths.
