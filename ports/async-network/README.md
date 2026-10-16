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
