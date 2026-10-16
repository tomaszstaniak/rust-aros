# Real async TCP consumer on older x86_64 ABIv11

Accepted 2026-10-16 on older AROS One pool v11-2, GCC10.5, AROS/sdk,
private aros-net-20261016 Rust 1.100.0-nightly (0dfb098f3). Both explicit
abort and unwind profiles pass the bounded consumer. See
[the native report](../../docs/evidence/2026-10-16-real-async/REPORT.md).

## Prepare and build

Supply a directory containing the original async-io-2.6.0.crate,
async-net-2.0.0.crate and polling-3.11.0.crate archives. `inputs.json` pins
archive checksums and the audited Zed reference revision/lock. This consumer
has its own versioned Cargo.lock; it does not reproduce the complete Zed graph.
Other registry sources must already be available in the selected private Cargo
cache. Clean sources, existing outputs, SDK and toolchain are never patched.

From the repository root, with explicit selected `aros-env.toml` and target JSON:

```sh
python3 ports/async-network/build.py "$CRATE_ARCHIVES" "$NEW_OUTPUT" \
  --toolchain aros-net-20261016 --cargo-home "$PRIVATE_CARGO_HOME" --profile unwind
```

The output must be new and outside the repository/archives. The recipe checks
all archive digests before creating output, rejects unsafe archive members,
applies versioned patches to copies and copies owned modules/libc/consumer.
It builds offline with --locked and the shared native panic-profile helper.
The default is abort. `prepare.json` records source hashes; `build.json` records
the compiler, sysroot, command, lock and binary digest. `built-not-run` is not
runtime acceptance. Generated source trees, libraries and artifacts stay out of Git.

## Runtime gate

Reserve the named machine, stage while stopped and extract privately to RAM:.
Use stack 1048576, numeric endpoints and fresh result filenames. Run the fixture
immediately before the consumer, not before boot/extraction:

```sh
python3 ports/async-network/fixture.py
```

In the QEMU user-networked guest, run `aros-async-consumer` with
`10.0.2.2:18761 10.0.2.2:18762`, directing stdout to a unique RESULTS: file.
The first endpoint is the controlled server; the second is checked closed.
Require the guest's final PASS marker and the fixture's completed87/errors[]
record. A Shell DONE marker alone is insufficient. Collect only while stopped.

The fixture accepts delayed, fast, cancellation and final completion requests.
It checks that the refusal endpoint actually refuses on the host before and
after the test. On this macOS, binding without listen drops SYNs: that is not
a refusal fixture. Never repair the guest based on that timeout.

## Backend contract

- polling3.11.0 uses validated tagged libc-aros socket handles and a separate
  Exec signal bit per selector. Pending notifications are preserved into
  WaitSelect; DOS pipes never enter the BSD fd_set. Level and oneshot are
  supported, including rotating delivery when the event capacity is small.
  Edge/urgent/HUP flags are not enabled; EOF is readable and connect errors
  are retrieved with SO_ERROR.
- Poller notify may run on other tasks. Add/modify/delete/wait require the
  opening task, checked before locking. Foreign selector drops defer signal
  cleanup to the owner; shutdown releases remaining signal allocations.
- async-io2.6.0 uses the owner-task driver, without the upstream fallback
  thread or its 500us handoff. Drive local concurrent futures with
  async_io::block_on on the socket-owning task. Nested calls panic explicitly;
  external executors and timer-only operation without an available SocketBase
  require separate integration/acceptance.
- Async handles are !Send and !Sync on this target, enforced by the compiler.
  Async-net TCP clones share the same local handle. Test notification workers
  are explicitly joined. Application notifier workers must finish before
  program shutdown; broader unjoined-thread exit semantics are not accepted.
- Numeric IPv4 is the accepted connect path. Hostname resolution and IPv6
  connect return Unsupported; async-net Unix APIs are excluded on AROS.
  UDP, listeners/accept, generic file/pipe async I/O and full smol/Zed remain
  outside this runtime gate. Other targets retain upstream implementations.

There is no rustix port claim: the AROS path removes its incompatible backend
from these selected crates, using the verified socket boundary instead. The
old File::try_clone/F_DUPFD_CLOEXEC gap remains separate.

## Stepped verified HTTPS

`std-groundwork/async-https` is a bounded transport consumer of the same
owner-task async-io backend. It is not a reqwest, rustls, full smol or Zed port.
Its opaque C interface hides Mbed TLS structure layouts. Each handshake call
advances one state, and read/write/close return WANT_READ/WANT_WRITE to the
reactor. Handles remain !Send/!Sync. Destruction releases memory without I/O;
explicit async close sends close_notify. Reusing a stream after cancelling a
write is outside this acceptance contract; cancelling construction destroys the
whole context. CPU work, including entropy startup and crypto, runs on the
owner task. An async deadline cannot preempt a synchronous C operation.

The backend is pinned apkg `91ac4d8f00bbaf5ac5b057322d3937a61acd1cd7`:
Mbed TLS3.6.7 and jitterentropy3.7.0. The build reads a Git archive of that
revision, its authoritative SDK source list, config and AROS adapters. It never
runs apkg's packaging script or writes its SDK. Jitter core retains -O0 and
-fwrapv; normal Mbed code uses -O2. apkg's gmtime port is accepted only under
this serial owner-task contract. Required chain/hostname/date validation has
no insecure option. Entropy failure has no fallback and occurs before connect.
Foreign constructors are rejected before accessing mutable owner-only TLS state.
A live-context count retains global PSA/jitter state until the last context is
freed. Only authenticated close_notify becomes successful TLS EOF.

Use a **new build output for each accepted run**; its UUID is compiled into
both variants and required by the report checker. Existing build/payload/
validation output is refused. Paths below are examples, not shared defaults:

```sh
python3 ports/async-network/tls-build.py "$archives" "$apkg" "$new_build" \
  --toolchain aros-net-20261016 --cargo-home "$cargo_home" --profile abort
python3 ports/async-network/tls-fixture.py "$new_certificates" --prepare
python3 ports/async-network/tls-stage.py "$new_build" "$new_certificates" \
  "$new_payload" --guest-dir RAM:rh/abort --prefix rust-https-unique-abort
```

Fixture preparation needs OpenSSL with x509 -not_before/-not_after support
(verified host OpenSSL4.0.3). It verifies the valid and all three negative
certificate cases on the host before running. Private keys stay in the host
fixture directory; the payload contains only root/other public CA certificates.
Test roots are explicit files and never installed in a host/guest trust store.
The deliberately broad 2020–2040 validity window avoids generation-time clock
skew; expired leaf is 2000–2001. The guest still validates its current date.

Follow the guarded VM pool stop/stage/start and screenshot/input procedure.
Extract the immutable payload into the exact private RAM path selected above,
check UnZip's return code/log, then run `Execute RAM:rh/abort/run-https` only
while the finite host fixture is ready:

```sh
python3 ports/async-network/tls-fixture.py "$new_certificates" \
  --run-id "$compiled_run_id" --deadline 180 > "$fixture_log"
```

The fixture binds loopback only (guest destination10.0.2.2), ports18771–18776.
It expects ten connections: four authenticated HTTP exchanges, three certificate
rejections before HTTP, cancelled pending ClientHello, established drop, and
truncated transport. Port18773 monitors the forced-entropy variant and must
receive zero connections. Once **both** guest binaries have completed, signal
only this fixture process with SIGINT; it joins bounded workers and writes the
final summary. Do not end monitoring before the forced-failure binary runs.
A hard deadline ends an incomplete run as failure. Stop the owned guest,
collect to a new directory, and release its lease.

```sh
python3 ports/async-network/tls-check.py "$new_build" "$collected" \
  rust-https-unique-abort "$fixture_log" --output "$new_validation"
```

The checker verifies both retained binary hashes, actual compiled source/header
copies, complete run UUID/start/end markers, zero shell return codes, all guest
cases, controller collection hashes and exact fixture counts. This detects
stale/missing/changed evidence; it is not an attestation against malicious
rewriting of manifests. Shared SDK/toolchain contents are identified, not fully
vendored or claimed hermetic. Guest binary bytes are not independently rehashed.
Do not accept a reused build UUID as a fresh independent runtime attempt.
