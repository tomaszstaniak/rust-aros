# Process and pipe acceptance fixture

One executable acts as parent and child. Run `process-stdio --case NAME
--report PATH`. Cases: status, args, exec-denied, spawn-error, wait, kill,
output, alloc-spawn, lsp, lsp-invalid, lsp-bytewise, lsp-invalid-bytewise.
Reports distinguish pass, failure and refused kill; a refused kill is only
reported after a self-bounded child has been reaped.

Build from this directory using a fresh isolated AROS std toolchain. Run host
tests from the repository root with `cargo +stable test --manifest-path
std-groundwork/process-stdio/Cargo.toml --offline` so the AROS config does not
select the cross target. Five unit tests cover bounded framing; the integration
test runs all twelve cases with real host processes.

## Guest payload

From the repository root:

```
python3 std-groundwork/process-stdio/tools/stage-payload.py /path/to/process-stdio /new/payload --prefix unique-run
```

Use that directory with the VM pool's stage operation while stopped. Start the
owned guest, unpack `POOLINPUT:PAYLOAD.ZIP` into `RAM:pr` with UnZip output
redirected, inspect extraction, then `Execute RAM:pr/run-tests`.
The staging helper refuses existing destinations and preserves mode 0755.
The guest script also sets `Protect RAM:pr/process-stdio +e` and records List
output. This matters: Shell can launch a file whose execute bit is absent,
while exec rejects it. Apply protection only to the RAM: copy.

The default script runs ten cases, including bytewise frame transport.
The exec-denied case prepares its file on RAM: because the pool's RESULTS:
FAT transport did not support the tested permission change. Add
`--include-buffered` to append ordinary buffered LSP cases; these are known to
block on the selected older v11 PIPE handler and need external supervision.
SCRIPT-RETURNED means the script returned from its commands, not that every
case passed: inspect individual reports and rc files.

`lsp-bytewise` bypasses Stdin's internal 8 KiB buffer with an exclusively owned
fd 0 in the child, and uses a one-byte framing buffer on each receiving side.
Frame bodies are read at their known length. This is a protocol workaround,
not a fix to generic buffered stdio, a production LSP client or a PTY.

Use an external supervisor for all guest runs: blocking I/O and waits have no
internal timeout. Raw diagnostics `posix-probe`, `c-spawn`, `c-spawn-signal` and
`c-spawn-search` report measurements, not pass assertions. The latter child
markers have fixed names and must only be used once per clean run generation.
`tools/trace-spawn.py` instruments a disposable std source copy; its toolchain
links only with this fixture, is timing/layout-sensitive, and must never be
used as a general compiler. Use a fresh build directory after changing rust-src.

See ../../docs/evidence/2026-10-15-process-stdio/REPORT.md for the initial
failures and its linked correction for the latest scoped native results.
