# Interactive LSP probes and coverage limits, 2026-10-15

## Scope

x86_64 ABIv11, older AROS One 1.3 UP, pool v11-1, QEMU pc/TCG qemu64,
two configured vCPUs, 2 GiB. GCC10.5 / AROS/sdk, isolated aros-exec-20261015,
Rust 1.100.0-nightly (0dfb098f3 2026-08-31). The actual WAT server uses the
[accepted explicit unwind profile](../2026-10-15-unwind/REPORT.md), upstream
0.11.0 at 428f74da7fcb85d2fca9bd9828f5ff7434c19af8, with the existing transport,
libc and inventory patches. The ordinary server source is unchanged. A separate optional diagnostic patch
was added for the final lifecycle measurement.

All guest work was in RAM:wl and reports in RESULTS:. Shared toolchains,
sysroots, SDKs, registry/upstream sources and guest runtime libraries were
unchanged. Each owned guest boot was stopped before collection; reservation
released after the final collection. Per-run payload-sha256.json files identify
the exact executed programs/scripts. Full stopped collections and manifests:
AROS/local/rust-lsp/results-04, results-05, results-06 and results-08.

## Client and validation

wat-interactive-acceptance continuously drains framed stdout on one thread and
stderr on another. It sends 12 symbol/cancelRequest/didChange cycles, accounts
for every expected ID and empty diagnostic notification regardless of order,
then checks first/last function names in a new final document before shutdown,
EOF, child wait and reader joins.

A null result or RequestCancelled (-32800) is allowed for stressed requests.
The counter is explicitly null_or_cancelled: it does not identify whether an
explicit cancellation notification or a pending write caused a particular
result. In the final probe at least one such result is required; an all-normal
run is insufficient interruption coverage, even when protocol behavior is valid.

The client-only Outbound helper coalesces up to three already-framed messages
into write_all/flush, bounded to 1 MiB wire size, retaining failure state so no
cleanup frame follows a partial write failure. This does not promise atomic
delivery or any scheduling order. The shared framing library is unchanged.

Six new unit tests pass: interleaved nullable replies/diagnostics; duplicate,
unrequested and malformed messages; nonempty diagnostics and hybrid envelope
rejection; accepted RequestCancelled; complete coalesced frame boundaries;
and partial-write terminal failure. Red → green was observed for the collector,
diagnostic validation and coalescing boundary. Two baseline client tests also
pass, as do seven transport tests, five framing tests plus the 12-case real
process integration, and all 23 Python tests. Logs are retained.

## Observed results

| Run | Workload | Result |
| --- | --- | --- |
| Original and patched host | 1024 functions, coalesced groups | 12 null results, empty diagnostics, final-version recovery, client/server0, stderr0 |
| native-04 | 128 functions, separate frame writes, earlier probe without coverage guard | 12 normal replies, empty diagnostics, final-version recovery, client/server0, stderr0 |
| native-05 | 1024 functions, separate writes, interruption required | 12 normal replies, empty diagnostics; coverage rejected, client1, server0/stderr0 |
| native-06 | 1024 functions, coalesced groups, interruption required | 12 normal replies, empty diagnostics; coverage rejected, client1, server0/stderr0 |
| native-08 diagnostic | same workload with deferred lifecycle trace | 12 cancellations applied after computation; coverage rejected, client1, server0, intentional trace on stderr |

The native-05/06 guard reports “all requests completed normally; interruption
was not exercised”. Those runs exit before the final recovery query/shutdown
request; cleanup sends exit, closes input, waits for the child and joins both
readers. They are not passing active-cancellation results. Native-04 supplies
the separate, smaller-document recovery result.

Summarized traffic records method/ID/version/size/result and diagnostic counts,
without dumping large document bodies. Native evidence is in native-04/05/06.
The later client revision also prints CASE_ERROR into stdout so future rejected
runs retain their reason in the file rather than only the guest Shell.

## Optional lifecycle trace

prepare.py --request-trace applies a separate versioned diagnostic patch.
The server stores at most 512 events in memory and dumps them to stderr after
the loop exits. Lookup hit, cancel_applied (after token.cancel), query_done,
write_ok/write_error and token removal are distinct events. Truncation has an
explicit marker. The client forwards --trace-requests through its argument
list; the staging helper enables it only for the interactive session.

Native-08 (diagnostic build) has the same 12 normal replies and client rc1
coverage rejection, with a healthy server exit0. The trace explains that run:

    registered -> worker_start -> query_done -> cancel_hit ->
    cancel_applied -> write_ok -> token_removed

This order holds for all 12 IDs 10–21. The token is found and cancellation is
applied, but after computation has completed. It therefore cannot change the
already computed response. The host diagnostic control instead records all
12 cancel_applied events before worker_start and returns null results, then
successfully validates recovery and shutdown.

There are no write_error events and no dropped trace events in either run.
Raw evidence: native-08/, request-trace-host.log and the two per-ID analysis
JSON files. Diagnostic stderr contains the intentional deferred trace and is
not expected to be empty.

The first trace draft was corrected before native measurement: a lookup hit
preceding token.cancel is not an applied-cancellation timestamp, and a returned
write is not necessarily successful. The corrected patch records both facts.
An earlier unused diagnostic boot was stopped without executing acceptance.

Deferred output avoids logging I/O during requests, but trace locking,
allocation and the diagnostic build still perturb scheduling. This observation
does not establish exact ordinary-server timing or general scheduler behavior.

## Diagnosis limits and next step

Increasing document size and reducing gaps between related writes did not
produce an interrupted result on the tested native profile. This does not prove
that notifications arrived after completion, that cancellation was ignored,
or that the scheduler/PIPE implementation is defective. The optional trace above supplies direct request/token lifecycle measurements
for its diagnostic run; ordinary-server timing remains uninstrumented.

The deterministic Salsa engine cancellation already passes natively with
unwinding; these results do not retract it. They expose a remaining gap in
wire-level active-cancellation coverage.

Next: obtain controlled active-query cancellation coverage and exercise
shutdown while requests remain outstanding. Do not keep changing load
and treat a successful process exit as proof that work was interrupted.

Receive deadlines are diagnostic only; blocked writes/wait/joins still need
the external VM supervisor because forced child termination remains open.
Shutdown with active outstanding requests, generic buffered PIPE, other v11
SDK profiles, v1 and arm64/Macaros remain unaccepted.

## Reproduction

Build the acceptance package from its own directory with the isolated AROS
toolchain and existing inventory patch config; use the unwind-enabled server.
ports/wat-server/stage.py --interactive-client PATH adds a new server/client
session with separate interactive output/rc after the baseline. The final
strict probe may return rc1 for insufficient native interruption coverage;
preserve that result rather than weakening the criterion.
