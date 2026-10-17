# Platform and Artcraft/Photocraft review

Checked 2026-10-17, read-only sibling review. No sibling source, SDK or system
configuration changed. Rust implementation stays x86_64 older ABIv11; v1 and
arm64/Macaros remain deferred.

## Changes since the previous Rust slice

Direct git ls-remote, without updating shared refs, returned v11 upstream
(deadwood2/AROS master)5b5fd4cfd4f039927c0d1bb9cdd7f4fc8644d23e, unchanged
from the cached review. Local v11 source remains5376f09bc1ed0c36a12fbb096406013f8a046e89.
Mainline upstream (aros-development-team/AROS master) is now
ae45f4644f80de542fb6af320c5d5fb5a1bc9925. A separate private bare audit of its
last30 commits identifies October15–16 DOS/security per-user assigns, account
changes, NameFromLock short-buffer cleanup, SFS validation/free-space changes,
Darwin interrupt masking limited to the AROS thread, and runtime documentation.
This is a bounded recent-history scan, not a full remote ancestry audit. These
changes are not installed in our local v1 SDK/guest or transferred to v11.

apkg HEAD661a8c6bd4d466300a13631bd1b44e6072aef131: package-relations/UI changes;
path-limited diff from selected91ac4d8f to HEAD for src/tls,third_party,sdk is
empty. Mbed TLS/jitter inputs stay pinned. Untracked -rexx.txt was left alone.
aros-xterm HEAD137cd384c580d7cd31070e8dd16b69e711848c46 is clean, last commit
October15, no October16+ change observed. Its recent console control/ordered
EOF behavior is relevant to future interactive terminal tests, not evidence of
new mmap or Rust socket functionality.

## Photocraft: what exists and what ran

Owned photocraft repo25cb76454a796ae5479cc44dc2a944790601a1da, clean work tree
aros-v11 e195936f1c32e1e2d7855724ff8dc9abb685546a, upstream pin
c450e755aed5e77fa6d4de7f04996085f5871085; versioned series0001–0021.
Its recorded preview acceptance covers older v11-2/AROS One1.3/QEMU,
GCC10.5/AROS-sdk/aros-net-20261016/unwind and rust-aros49324c4: CLI conversions,
GUI edit/save/reopen and MCP stdio41/41, with prior suite45/45. Those are sibling
results, not newly repeated application runs in this Rust change. Some overview
text predates later fixes; inspect the patch series and latest acceptance.
Private application evidence stays in its containing workspace, not copied here.

Tokio 1.53.1, tokio-util 0.7.19, rmcp 3.5 retain runtime/time/sync/stdio paths.
The automation Cargo manifest separates mcp from bridge=[mcp,tokio/net].
The accepted MCP/stdio runs do not accept Tokio-net, mio, socket2 or serve --port.
Existing bounded frame/short-read/stdin retirement/worker-join adaptations are
worth reusing; client close remains part of the stdin retirement contract.

The injected connector in automation/src/bridge.rs already isolates transport,
but its futures and I/O halves require Send and the connector requires
Send+Sync. Our socket-owner !Send handles cannot be inserted directly. A
separately accepted actor on the socket-opening task with Send channel proxies
is one candidate; a different owning task needs explicit SocketBase initialization
and lifecycle design first.
a reviewed local-only connector API is another. Neither is implemented by this
audit. Listener/accept, auth deadlines, shutdown and GUI-thread dispatch need
native tests. Preserve bridge mutation no-replay behavior when retrying.
Application pthread detach-after-finish, condvar clock and Rayon teardown patches
also merit isolated regressions before any Rust-runtime backport.

## mmap: confirmed boundary bug, not a guessed font workaround

Fontique 0.11.1 uses memmap2 0.9.11 and read-fonts 0.41.0. Its font.rs37–38
calls Mmap::map then FontRef::from_index, discarding errors via .ok()?. scan.rs
and source_cache.rs have equivalent hidden mapping failures. The prior GUI
observation of a selectable system font rendering a built-in face did not
prove a cause. No Fontique fallback patch was applied here.

Current Rust _SC_PAGESIZE was8, while the selected SDK enum is54 (8 denotes
_SC_2_PBS_CHECKPOINT). The checked SDK sysconf implementation returns-1/EINVAL
for page size. The existing sysconf_pagesize helper was not the function Rust
called. memmap2 casts-1 to usize; its Drop cannot reconstruct the original
allocation address. Since munmap then fails, our copy-map table retains slots.

The new exact native probe reproduces this independently: JetBrainsMono
270224bytes parses to1754 glyphs; page=-1; repeated map/drop fails at iteration64
with ENOMEM. With the corrected selector and private rs_sysconf mapping-granule
answer,128 iterations and offset/drop/capacity/reuse checks pass. This confirms
the adapter lifecycle bug. It does not prove the GUI's entire font-selection
problem is fixed. memmap2 and Fontique sources remain unmodified.

The adapter remains a read-copy implementation, limited to64 simultaneous maps.
The fix reports its4096 allocation granule and delegates other sysconf selectors
to the SDK. It adds no VM-backed sharing/protection, writable shared mapping or
concurrent pread-on-the-same-file-position acceptance.

## Next application-driven scope

1. Rebuild a private Photocraft against the fixed boundary and test the actual
   Fontique scan/cache/render paths with explicit diagnostics before declaring
   the font symptom closed.
2. Isolate the pthread/condvar lifecycle fixes as runtime regressions.
3. Design owner-task listener/accept and Send channel transport for the existing
   connector seam, then accept the real bridge. General Tokio-net remains a
   separate implementation target; a compiling cfg is not a runtime result.

The common Rust acceptance gate adds the matched font mapping diagnostic to
socket/async/TLS/entropy/process/LSP coverage before application integration.
