# Rust for AROS renewal: design and implementation brief

Date: 2026-10-15. Status: approved; first M0 slice implemented. See docs/current-state.md in the isolated checkout.

## Intended outcome

Make Rust for AROS reproducible and useful for substantial applications. Use
Zed as a reference workload for the quality of platform support, not a promise
to deliver a complete Zed port in this project. The current agent implements
the work directly; delegation is not requested.

Platform priority, explicitly confirmed by the user:

1. AROS ABIv11 is primary.
2. AROS ABIv1/mainline follows once the relevant v11 milestone is validated.
3. Support x86_64 and arm64 through hosted Macaros. Macaros is not currently
   working reliably on the local host. Report arm64 compile, link and runtime
   results separately; a missing runtime is not grounds to drop arm64 support.

Native Raspberry Pi is outside this scope. Do not treat its SDK, toolchain or
results as interchangeable with hosted Macaros. Record Macaros's actual ABI
and SDK instead of inferring them from the primary v11 policy.

Host development means Cargo-based cross compilation, build scripts and proc
macros running on the host, explicit SDK/compiler selection, isolated sysroots,
and repeatable deployment and evidence collection. Running rustc or Cargo
natively on AROS is a separate future question, not an initial requirement.

## Verified starting point

Read-only inspection on 2026-10-15:

- Repository: `/Users/tumash/Work/AROS-dev/rust-aros`.
- HEAD: `8d35fda`, dated 2026-09-24. Twelve commits ahead of the locally cached
  origin/main (`1e26a71`). Only `.DS_Store` is modified in the checkout.
  Remote SHA could not be verified: shell Git access failed DNS resolution.
- `aros-nightly` executes rustc `1.100.0-nightly (0dfb098f3 2026-08-31)` and
  resolves to `/Users/tumash/Work/AROS-dev/aros-toolchain-sysroot-8d35fda`.
- Existing no_std/alloc runtime, SDL2 bindings, argument and DOS filesystem
  helpers are used by Burntime. Experimental std has filesystem, threading,
  process and blocking TCP examples, ripgrep and a limited Tokio example.
- The last std fix makes sockets optional at startup on the tested v11 setup.
  Test evidence is in `aros-markedit/docs/attachments/2026-09-24-toolchain-8d35fda/`.
- `libc-aros/csrc/compat.c` still defines getentropy using clock-seeded random().
  This is an actual implementation finding, not a claim that AROS lacks entropy.
- `libc-aros/build.rs` still searches TARGET for a JSON filename, although the
  runtime crate already documents why Cargo's TARGET is a target name. It also
  hardcodes crt before net, which cannot simply transfer to mainline.
- Top-level README contradicts itself about ripgrep/Tokio and uses an unpinned
  nightly install recipe. Existing installer replaces bin/lib in its selected
  sysroot and relinks the shared toolchain; do not run it on the shared setup.

These are source/install observations and historical runtime evidence. No fresh
AROS runtime acceptance was performed during this inspection.

## Reuse current sibling projects

### arospkg / apkg

Source: `/Users/tumash/Work/AROS-dev/arospkg`.

- `src/tls/entropy_jent.c`: process-local jitterentropy collector, serialized
  with an Exec semaphore, initialization and health failures propagated, output
  wiped after read failure, explicit shutdown.
- `src/tls/jent-aros/jitterentropy-base-user.h`: timestamp and memory adaptation
  for x86_64, i386 and aarch64; header presence alone is not runtime acceptance.
- `third_party/jitterentropy/`: pinned upstream 3.7.0, unchanged upstream sources.
- `sdk/README-jitterentropy.txt`: reuse contract and scoped x86_64/v11 evidence.
  Compile the jitterentropy core with upstream-required -O0. The documented
  raw-noise run is not a completed restart assessment or certification.
- `src/tls/entropy_getentropy.c`: system source path used by mainline/Pi builds;
  chunks reads to the system API's 256-byte limit and fails on error.
- README documents Mbed TLS 3.6.7 plus jitterentropy 3.7.0 and multiple targets.

Use the existing library/platform adaptation, with pinned provenance and
licences. Do not invent a second jitter implementation. A POSIX getentropy
adapter must implement its own return/errno/length contract, not expose an
Mbed-TLS-specific callback as if it had POSIX semantics. Check Rust getrandom
versions independently: fixing libc alone does not enable every crate backend.
Select the source by verified platform configuration. Never fall back to
clock/rand after initialization or health failure.

### aros-xterm / Limpet

Source: `/Users/tumash/Work/AROS-dev/aros-xterm`.

- README identifies terminal 0.4.1 and SDK 0.3.0; consult actual source and
  artifact hashes for the pair used by each new test.
- `sdk/README.md`, `doc/capabilities.md`, `doc/sdk/api.md`: presentation query
  on the actual output handle, colours, geometry, links, progress, title and
  full-screen input capabilities. Preserve plain output on CON: and files.
- `docs/crossterm-pty-resize-20260927.md`: Ratatui 0.29 / Crossterm 0.28.1
  acceptance on mainline x86_64 with an isolated experimental posixc runtime.
- `docs/backlog/crossterm-current-runtime.md`: that milestone is closed;
  mainline socket-startup and upstream-runtime follow-ups remain separate.
- `tools/rust-tty/`: inspect and consolidate the existing Rust adaptation
  before making another fork.

Keep released v11 Limpet SDK integration separate from experimental mainline
PTY/termios acceptance. Neither proves unmodified Crossterm works on stock v11.
Do not reopen closed mainline milestones or install experimental posixc into
shared guests to make a demo pass.

## Zed reference workload

Upstream source inspected through the web on 2026-10-15:

- https://github.com/zed-industries/zed/blob/main/Cargo.toml
- https://github.com/zed-industries/zed/blob/main/rust-toolchain.toml
- https://github.com/zed-industries/zed/blob/main/crates/gpui/Cargo.toml
- https://github.com/zed-industries/zed/blob/main/crates/gpui/src/platform.rs
- https://github.com/zed-industries/zed/blob/main/crates/gpui_platform/Cargo.toml

This was a moving-main inspection, not a pinned dependency audit. Obtain and
record a specific revision and its lockfile before compiling representative
crates. The API request for main's commit failed during this inspection.

The workspace includes smol/async-io/async-process as well as Tokio, a Zed
reqwest fork with rustls/native roots, file watching, SQLite and tree-sitter.
Therefore Tokio networking alone is not the definition of Zed readiness.
GPUI exposes a platform interface encompassing executors, text, event loop,
windows and displays. Rendering and native integration need their own study;
a Zune wrapper or terminal backend is not automatically a GPUI backend.

Track these independently, naming exact versions and enabled features:

| Area | Representative acceptance |
| --- | --- |
| Host cross-build | host proc macro/build.rs plus target C FFI in a clean clone |
| std baseline | args, Unicode paths, files, threads, time, error formatting |
| Randomness | successful source read plus forced initialization/read failures |
| Processes / LSP | child stdin/stdout/stderr, EOF, exit status, cancellation and cleanup |
| Async I/O | real readiness, wakeup, nonblocking semantics, close and cancellation |
| HTTPS | CA validation, hostname/expiry rejection, source-failure rejection |
| Files / watching | save/rename semantics, directory changes and watcher lifecycle |
| Native dependencies | pinned SQLite/tree-sitter build and actual target execution |
| Terminal | released v11 Limpet integration; separately scoped PTY experiments |
| GPUI | explicit backend inventory and a minimal window/text/input probe |

Inventory Git support, dynamic loading/extensions and language-server binaries
as additional requirements; classify rather than assume their availability.

## Selected approach and milestones

Use incremental platform support driven by real consumers. A direct full Zed
build would entangle GUI and runtime failures. Generic cleanup alone would not
prove application support. Each milestone must leave a runnable bounded result.

### M0: reproducible x86_64 v11 baseline (first implementation scope)

- Work in an isolated writable copy; preserve existing consumers and toolchains.
- Pin the exact known nightly before considering any toolchain upgrade.
- Centralize explicit ABI, SDK, compiler, archiver, target, build and sysroot
  configuration. Reject mismatched or missing selections with clear errors.
- Make sysroot creation use an explicit destination and local toolchain name;
  refuse destructive reuse and avoid relinking shared aros-nightly by default.
- Correct stale README claims; describe arm64/Macaros as an intended profile
  with explicit current validation limits.
- Run clean-build baseline probes from existing examples, with separate output
  directories and file-based evidence. Record missing downloads or inputs.

Acceptance: a second isolated destination can reproduce the selected v11 build
without changing shared sysroots; host/target compilation are distinguished;
existing baseline runtime cases have fresh named evidence or explicit blockers.

### M1: real entropy and ecosystem randomness

Reuse arospkg jitterentropy for the selected v11 profile; inspect the actual
system API before choosing the mainline or Macaros source. Test initialization,
length limits, repeated/concurrent access, failure propagation and lifecycle.
Then validate the getrandom/rand versions required by the pinned reference.

### M2: v11 terminal integration

Wrap/reuse the released Limpet SDK, with capability detection and graceful CON:
and redirected-file behaviour. Validate Unicode, geometry, resize/input modes,
Ctrl-C, outstanding-read cancellation and terminal restoration. Reuse existing
Ratatui work where applicable. PTY public-ABI work remains explicitly separate.

### M3: processes and async networking

Prioritize a bounded LSP-style child/pipes scenario and an async network client.
Choose adapters from the pinned consumer dependency graph (including smol),
rather than precommitting to Tokio only. Verify thread/task cleanup, EOF,
timeouts, backpressure and cancellation. Integrate HTTPS using an explicitly
selected backend; Mbed TLS success does not establish rustls compatibility.

### M4: representative Zed components and GPUI feasibility

Probe pinned portable text/parse components, SQLite, filesystem/watch and process
clients independently. Identify the smallest useful GPUI probe and its renderer,
font, window/event-loop and clipboard requirements before backend implementation.
Report demonstrated support and unresolved dependencies instead of a percentage.

Apply each validated v11 milestone to the next v1 profile. Maintain arm64
Macaros compile/link probes in parallel where inputs are available; execution
acceptance waits for a functioning named hosted system. Do not make v11 progress
depend on repairing Macaros.

## Validation and boundaries

Read AROS-dev/AGENTS.md, coding standards, platform notes and the current
testbench rules before implementation/deployment. Review recent relevant v11
and v1 upstream changes before platform-dependent decisions. Tests for behaviour
changes precede production code and cover meaningful failures. Do not add tests
merely mirroring documentation or implementation details.

Every report records architecture, actual ABI, source revisions, nightly,
compiler/SDK, link inputs, binary hashes, machine and exercised cases. Include
what the run did not establish. Never equate compile, link and execution.

No publication, shared SDK/sysroot replacement, guest-system modification or
full Zed port is authorized by this design. Prepare reviewable artifacts first.
The existing sandbox permits writes under AROS, not AROS-dev; use an isolated
copy under the writable workspace until placement is deliberately resolved.

## Continuation prompt

Implement Rust for AROS renewal using this document as the design brief. Keep
ABIv11 primary, ABIv1 next, and support x86_64 plus hosted arm64 Macaros with
honest runtime limits. The current agent implements directly. Start with M0;
create a detailed implementation plan and execute it after design review. Reuse
arospkg's entropy/TLS work and Limpet's released SDK and Rust experiments.
Pin a Zed revision as the reference dependency graph, then advance M1-M4 in
bounded, tested increments. Do not replace real errors with successful stubs,
claim absent capabilities from stale notes, or change shared installations.
Keep the durable current state, evidence and next acceptance criteria updated.
