# Rust for AROS: status (2026-11-04)

This main branch consolidates the shared runtime, x86_64 ABIv1 and ABIv11,
and native AArch64 ABIv1 profiles. Application fixes belong here, not only
in application-specific branches. Profile-specific choices remain explicit.

| Profile | Recorded qualification |
|---|---|
| x86_64 ABIv11 | std/runtime probes under QEMU; Helix aros3 and wat_server on AROS One 1.3 |
| x86_64 ABIv1 | C/no_std/std and Cargo main under QEMU; later Helix candidate 6 acceptance on nightly 20261031 |
| AArch64 ABIv1 | common std and thread/runtime probes under QEMU; physical Raspberry Pi acceptance still pending |

These are historical, configuration-specific results, not a fresh native test
of this consolidated checkout. Do not mix SDK headers and runtime versions.
The Helix aros3 v1 build was built but not run. Its v11 sysroot ancestry is
not fully established; publishing the source does not establish reproducibility
of that installed toolchain.

## Included work

- Native AArch64 PAL imported from pinned Jonx sources with shared build and
  acceptance tooling, TLS/thread cleanup and Cargo-main startup work.
- mmap page accounting, entropy/TLS/process probes and runtime regressions.
- x86_64 mainline profile and SDK-specific libc layouts, including the newer
  ABIv1 pthread_attr_t size and corrected C type alignments.
- CRT path/descriptor serialization, file locking and console geometry work.
- AROS home_dir uses nonempty HOME only, avoiding the user-database fallback
  that triggered a C-runtime lifetime bug. This is a deliberate limitation.
- wat_server port and build-path remapping, used by
  [Helix aros3](https://github.com/tomaszstaniak/aros-helix/releases/tag/aros3).

## Separate build experiment, same repository

The pinned stock-compiler/mainline SDK integration is preserved in
`patches/mainline-official`, `profiles/mainline-official` and the official
qualification tools. It has build/link evidence only; runtime is UNTESTED.
It is not the toolchain used to produce Helix aros3. Its PR metadata records
an October observation, not current upstream PR status.

## Remaining limits

Tokio networking is not generally qualified; successful runtime, timers or
stdio do not establish socket support. AArch64 SocketBase ownership remains
open. Forced process termination is not a portable supported contract.
Creating a first TLS key inside a destructor remains outside acceptance.
Runtime behavior depends on the chosen SDK/system. Helix-specific fixes to
parking_lot_core and terminal/event handling live in the Helix port's series.

## Verification and provenance

Local development history is retained. This publication consolidates source
from helix-lsp-wat (6dca821), which contains the common runtime/ARM work,
and the disjoint official-profile changes from c10cf0b. The c1b5e67 HOME-only
change on the older sysroot branch is already represented by 321495a; no
second implementation is introduced. The mainline-profile README is replaced
by this combined status rather than overwriting the application profile.

Private raw evidence, workstation paths and debugger dumps are not distributed.
Older integration ledgers may refer to those private records. This source sync
does not claim that all older build artifacts can be reproduced bit-for-bit.

Run host checks with `python3 -m unittest discover -s tests -v` and
`cargo test --manifest-path acceptance/Cargo.toml`. The official-profile tests
additionally require a prepared source tree via AROS_OFFICIAL_SOURCE.

## Consolidation checks

On 2026-11-04 the consolidated source passed 108 Python host tests, 20
official-profile tests against the prepared pinned source, and the Rust
acceptance workload test on the host. These are not fresh native v1/v11
or Raspberry Pi runs. The WAT build now rejects ambient native flags before
querying the compiler, and its validator tests use public protocol-only
fixtures instead of private native-run directories.
