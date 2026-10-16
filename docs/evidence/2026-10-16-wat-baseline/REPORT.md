# Repeatable v11 WAT baseline build and validation, 2026-10-16

## Scope and inputs

x86_64 ABIv11, older AROS One pool profile v11-2 under QEMU, GCC10.5 /
AROS/sdk, isolated aros-exec-20261015. Compiler pinned to Rust
1.100.0-nightly (0dfb098f3 2026-08-31). WAT 0.11.0 source pinned to
428f74da7fcb85d2fca9bd9828f5ff7434c19af8 with the existing transport,
worker-lifecycle, inventory and optional diagnostic checkpoint patches.
The 2026-10-15 platform review is reused; this slice changes host build/report
wiring, not AROS components or the previously accepted runtime adapters.

## Common entry point

ports/wat-server/baseline.py build prepares pinned sources, builds the ordinary
server, copies/builds acceptance packages and stages a guest bundle. Explicit
--profile unwind also builds the diagnostic server and two independent probes.
Omitting --profile selects abort and stages only the ordinary baseline. Output
must be new and outside upstream; all targets are run/profile-owned. Compiler
cfg is checked against the requested panic strategy. build.py shares the same
native profile helper and retains --unwind as a compatibility alias.

Both native profiles reject five ambient flag/panic overrides. Client/transport
sources and locks are copied before building; no original Cargo.lock changes.
libc-aros is copied to build-owned inputs and source-hashed. The resolved private
sysroot and full Rust library source hash are recorded. Port/config hashes are
captured before building and checked again at the end along with std sources.
The pinned-source prepare script and checksum-validated inventory patch remain
responsible for their existing inputs. run.json preserves exact commands/logs,
SDK/tool paths, input identities and payload hashes. Build success has status
built-not-run; it never implies runtime acceptance.

baseline.py check reads a stopped collection and writes validation.json. It
requires all case codes/markers and exact active teardown ordering, and returns
nonzero on a missing or failed report. A failed recheck replaces earlier passed
summary status. VM reservation, focus inspection, execution, supervision and
stopped collection remain the shared testbench workflow; this tool does not
silently acquire, interrupt or replace another owner's machine.

## Stale result protection

Review identified a false acceptance path: a new start receipt could coexist
with an old DONE and old successful reports after a partial same-prefix run.
The final script refuses any preexisting required suite output before writing
new results, includes a unique per-build UUID file in the payload manifest,
and copies matching start and completion receipts. The checker requires both.
Tests observed the stale-receipt case fail before the fix and pass afterward.

Native retry of the completed unwind script is rejected with rc20 before a new
suite starts. The guard message was inspected in the guest screenshot retained
at AROS/local/rust-lsp/unified-repeat.png; native-unwind/wu16b-repeat.rc records
20. AmigaDOS Execute did not retain its internal Echo in the redirected retry
file, so the empty repeat.txt is not presented as textual proof of that message.
The original suite still passes the complete checker afterward.

Receipts identify the copied staged manifest; they do not constitute an
independent guest-side executable rehash. The input ISO is immutable and the
host run manifest/payload hashes identify what the pool staged. No stronger
executed-byte claim is made.

## Actual validation

Two fresh final outputs were built and run: baseline-unwind-02 (wu16b) and
baseline-abort-02 (wa16b), both under AROS/local/rust-lsp. Their full run.json,
logs and validation.json are retained in build-unwind/ and build-abort/.
Their input hashes preserve the executed build-time files; the only later
driver edit removes trailing whitespace, without changing commands or behavior.

| Profile | Native scope | Result |
| --- | --- | --- |
| explicit unwind | ordinary server baseline; checkpoint cancellation/recovery; outstanding shutdown; active exit; active EOF; independent typed unwind/destructor/thread test; deterministic Salsa cancellation | all seven cases rc0, checker passed |
| default abort | separate ordinary server/client baseline, including 16 outstanding mixed requests and orderly shutdown/EOF | rc0, checker passed; no cancellation acceptance claimed |

The baseline uses the distinct uninstrumented server. Diagnostic sessions have
the intentional deferred trace. Active exit/EOF retain cooperative checkpoint
cancellation, response accounting, worker completion/joins, child wait and both
client reader joins. The independent unwind program asserts all 67 destructors
and catches worker/rethrow payloads as previously specified. No gate timeout,
write error, dropped trace or unexpected panic occurs.

All 33 Python tests pass (23 existing plus 10 baseline/report/profile cases).
The tests cover real retained report acceptance, missing/failed cases, missing
probes, trace truncation/order, receipt mismatch/completion, ambient overrides,
upstream-write rejection, and replacing stale passed summaries. Red-to-green
was observed for report validation, completion receipts and additional Cargo
override rejection. Existing Rust transport/client/process sources are
unchanged; their native paths are exercised by the generated bundle.

The existing nonfatal host rust-objcopy/libLLVM and Cargo build-std patch
warnings remain in build logs; commands complete0. No shared SDK/toolchain/
sysroot, clean upstream checkout or registry source was replaced. Guest work
ran from RAM:wl and only fresh wu16b-/wa16b- result files were written. Each
collection happened while stopped; the v11-2 reservation was released.
Full verified stopped collections: AROS/local/rust-lsp/results-11 and results-12.
Only this slice's files are copied here. Initial disposable development builds
and an unused boot are not counted as runtime acceptance.

## What this did not show and next step

This closes the scoped repeatable x86_64 v11 LSP baseline gate. It is not full
M3: async readiness, networking and selected HTTPS backend acceptance remain.
It does not install or hermetically distribute GCC/SDK/sysroot, promise
bit-identical output across hosts, prove natural-load/mid-query interruption,
or add forced termination for blocked writers/noncooperative workers. Other
SDK/runtime combinations, v1 and arm64/Macaros remain deferred.

Reproduction commands and VM handoff are in ports/wat-server/README.md under
“Unified native baseline entry point”. Next: select and validate the smallest
async/network consumer path from the pinned reference dependency graph, with
real readiness/wakeups and an explicitly chosen TLS backend.
