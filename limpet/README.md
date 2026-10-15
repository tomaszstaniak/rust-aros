# aros-limpet

An allocation-free Rust wrapper for Limpet SDK 0.3.0. The crate itself is no_std
and does not install an allocator or panic handler, so std and no_std programs
can both use it. Package name: aros-limpet.

The first API covers presentation detection, geometry snapshots, ANSI/RGB
sequence generation, window titles, ordinary output and progress lines. It does
not implement raw keyboard input, PTY, termios or a Ratatui backend.

```rust,ignore
use core::fmt::Write;
use aros_limpet::Output;

// The current DOS output stays open, and no other task uses it concurrently.
let mut out = unsafe { Output::current() };
out.set_ansi(2, false)?;
writeln!(out, "Ready")?;
out.reset_colors()?;
out.progress("Reading files")?;
```

The query targets the actual output handle, not Input(). Unsupported consoles,
files and failed/malformed replies get zero capabilities: decoration writes
nothing and progress prints ordinary lines. Plain text still writes normally.
Buffers from std/C stdout must be flushed before the query; writes through this
wrapper go straight to DOS. Query and write operations block; DOS has no query
timeout. Geometry is a snapshot, not automatic resize notification.

Output::current is unsafe because foreign DOS handle lifetime and concurrent
access cannot be enforced by Rust: keep the captured process-owned handle open
and unchanged, and prevent access from another task while these calls execute.
Output is neither Send nor Sync, but independently capturing the same inherited
handle from another task is still possible. The wrapper does not close the
process-owned handle and does not write terminal resets from Drop.

Colours preserve ANSI meaning when Limpet uses Workbench pen semantics.
Unknown colour levels produce no sequence. Titles are capability-gated and
reject embedded NUL, control characters and overlong UTF-8. Progress rewriting
requires both carriage return and erase-to-end-of-line to be confirmed.

On AROS, set AROS_GCC, AROS_SDK and (for non-GCC drivers) AROS_AR. The existing
setup.sh supplies them to the examples. Cross ABI results do not transfer.
On the host, cargo test compiles the SDK's pure C code and injected transport
checks; optional HOST_CC/HOST_AR select host tools. No crates.io dependencies.

Examples: ../examples/limpet (no_std), ../std-groundwork/limpet-std (std).
Acceptance evidence: ../docs/evidence/2026-10-15-limpet/REPORT.md.
