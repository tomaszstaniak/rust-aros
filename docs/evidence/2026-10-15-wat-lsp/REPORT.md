# Real language-server acceptance, 2026-10-15

## Scope

Native x86_64 ABIv11, older AROS One 1.3 UP pool v11-1, QEMU pc/TCG,
qemu64, two configured vCPUs, 2 GiB. GCC 10.5 from AROS/toolchain with
AROS/sdk; this is not the v11 2026.09 SDK/guest profile.
Rust 1.100.0-nightly (0dfb098f3 2026-08-31), isolated production
aros-exec-20261015, private Cargo cache and build outputs under
AROS/local/rust-lsp. Shared sysroots, SDK, registry sources and guest libraries
were not changed. Guest binaries and work were in RAM:wl; only reports were
written to RESULTS:. Both test boots were stopped and collected, then the pool
reservation was released.

Pinned workload: upstream g-plane/wasm-language-tools wat_server 0.11.0,
commit 428f74da7fcb85d2fca9bd9828f5ff7434c19af8, MIT.
The original host server, patched host server and native server use the same
analysis sources and upstream dependency versions, except explicit AROS libc
and inventory patches described below.

## Changes

transport/ is the dependency-free aros-lsp-transport library. It owns bounded
Content-Length readers/writers with explicit Buffered/DosPipe policies.
DosPipe reads headers one byte at a time from an unbuffered input and bodies
at their declared size. Limits cover headers and bodies on both sides; any
read/write/flush failure poisons the session object. EOF is clean only before
a new header. JSON semantics remain with the caller. process-stdio now reuses
this parser, retaining the existing buffered comparison and process cases.

The WAT transport patch changes the server's stdin owner on AROS to an
exclusively owned fd 0 File, uses the shared frame parser, returns terminal
input errors, and keeps one mutex-protected FrameWriter across worker
responses. Retaining that writer matters because upstream workers discard
output errors: a new writer per response could append after a partial frame.

The application build explicitly resolves libc-aros 0.2.189 instead of
crates.io libc 0.2.186, whose AROS compilation failed on an unresolved unistd
module. Existing std dependencies already used the isolated libc port.

## Runtime failure and regression

Run 01 is retained as failure evidence. initialize returned real capabilities,
then didOpen aborted with:

    ingredient 'salsa::input::JarImpl<wat_service::document::Document>' was not registered

The captured panic uses backticks around the ingredient name; see raw evidence.
The child was reaped with status 20 and stderr captured. The abort requester
was dismissed before collection.

inventory 0.3.24's constructor cfg omits AROS, so its retained __CTOR pointer
has no constructor section. A C constructor compiled with the selected GCC
emits .ctors. The versioned inventory-aros.patch selects that section only for
x86_64 AROS. Run 02 adds an independent inventory registration assertion before
main's first server spawn and runs the complete Salsa workload successfully.

The source crate archive SHA-256 is the upstream Cargo.lock value:
a4f0c30c76f2f4ccee3fe55a2435f691ca00c0e4bd87abe4f4a851b1d4dac39b.
Extraction and patching occur in a temporary directory followed by rename.
The complete patched tree is checked on every reuse, including existing
directories. Upstream exports and registry sources remain untouched.

## Native results

Run 02: client rc 0, server status 0, stderr 0 bytes, final DONE marker.

| Check | Result |
| --- | --- |
| inventory registration before main | PASS |
| initialize with real upstream capabilities | PASS |
| didOpen invalid WAT: syntax diagnostics | PASS |
| didChange repaired document with UTF-8 export name: diagnostics clear | PASS |
| formatting: real edit ranges applied with UTF-16 positions, content preserved | PASS |
| document symbols: upstream worker-thread analysis returns function | PASS |
| shutdown response, exit, stdout EOF, child wait, stderr join | PASS |

Actual JSON traffic and permissions are in native-02/. native-01/ preserves
the earlier failure. payload-sha256.json identifies the exact executed
binaries/scripts, including the failed run. Full stopped-disk collections with
manifests remain at AROS/local/rust-lsp/results-01 and results-02.

## Host and recipe verification

- Original upstream and patched host servers both pass the same real client;
  raw traffic: wat-reference-final.log and wat-port-final.log.
- Seven transport tests cover the open-DOS-pipe read-size model, coalesced
  UTF-8/empty frames, malformed/truncated input, bounds, partial writes,
  per-frame flush, and terminal write failure.
- Existing process regression passes five framing tests and one integration
  containing 12 actual host process cases.
- All 23 Python installer/setup/entropy/process contract tests pass.
- Clean pinned prepare and offline native compile/link pass. Repeated builds
  validate the existing patched dependency before reuse.
- Negative checks reject a matching lockfile without the pinned source tree
  and a modified/incompletely prepared inventory tree.
- Lock comparison: only libc 0.2.186 → local 0.2.189, local inventory source,
  local transport addition and wat_server's transport dependency changed.
  Other upstream package versions remain unchanged.

Host rust-objcopy still reports its existing nonfatal missing libLLVM.dylib
warning. No host tool installation was changed.

## Reproduction and limits

See ../../../ports/wat-server/README.md for exact preparation, isolated build,
client and execute-preserving VM staging commands. The prepared source tree
and generated build/dependency directories are disposable outputs outside Git.

This accepts a real synchronous language-server session on the matched v11
profile with an explicit DOS read policy. General buffered PIPE/stdin,
timeouts/forced child cancellation, signals, concurrent request cancellation,
async readiness, filesystem watching, rust-analyzer and Zed/GPUI remain open.
No v1, newer v11 SDK/guest, arm64 constructor or Macaros runtime result is
inferred from this x86_64 test.
