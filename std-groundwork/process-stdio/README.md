# Process and pipe acceptance fixture

One executable acts as parent and child. Run `process-stdio --case NAME
--report PATH`. Cases: status, args, spawn-error, wait, kill, output, lsp,
lsp-invalid. Reports distinguish pass, failure and refused kill; a refused
kill is only reported after a self-bounded child has been reaped.

Build from this directory using a fresh isolated AROS std toolchain. Run host
tests from the repository root with `cargo +stable test --manifest-path
std-groundwork/process-stdio/Cargo.toml --offline` so the AROS config does not
select the cross target. Five unit tests cover bounded framing; the integration
test runs all eight cases with real host processes.

Use an external supervisor for guest runs: blocking I/O and waits have no
internal timeout. Framing is a test fixture, not a production LSP client.
See ../../docs/evidence/2026-10-15-process-stdio/REPORT.md for native failures;
host success does not establish AROS process support.
