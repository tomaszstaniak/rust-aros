# Rust for AROS: status (2026-11-04)

This main branch consolidates the shared runtime, x86_64 ABIv1 and ABIv11,
and native AArch64 ABIv1 profiles. Application fixes belong here, not only
in application-specific branches. Profile-specific choices remain explicit.

| Profile | Recorded qualification |
|---|---|
| x86_64 ABIv11 | std/runtime probes under QEMU; Helix aros3 and wat_server on AROS One 1.3 |
| x86_64 ABIv1 | C/no_std/std and Cargo main under QEMU; later Helix candidate 6 acceptance on nightly 20261031 |
| AArch64 ABIv1 | common std and thread/runtime probes under QEMU; physical Raspberry Pi acceptance still pending |
| AArch64 ABIv1, unix (`aarch64-v1-raspi-unix`, 2026-11-14) | std probe under QEMU raspi3b on nightly 20261110: arguments, environment, files and directories, metadata, sleep, four threads with a channel, condition variable with and without timeout |

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

## Unix std on AArch64 (2026-11-14)

`profiles/aarch64-v1-raspi-unix.json` is the AArch64 counterpart of
`x86_64-v1-pc`: `target-family = unix`, `target_abi = abiv1`, std's unix
paths over posixc and the same `libc-aros` bindings. Programs written for
`cfg(unix)` (command-line tools, their crates) build for the Raspberry Pi the
way they do for PC. The older `aarch64-v1-raspi` profile keeps the imported
PAL and its C bridge; the common sysroot serves both and selects the PAL only
when the target is not unix.

The mainline C headers of both architectures are identical. Of 130 measured
sizes, alignments and offsets only `pthread_mutex_t`, `pthread_cond_t` and
`pthread_rwlock_t` differ (136, 152 and 120 bytes on AArch64). The native
toolchain has no unwinder library: `libc-aros` supplies weak entry points that
report an empty stack, so a panic aborts without a backtrace. C build scripts
that derive a Clang `--target` from the Rust target get an `abiv1` environment
Clang does not know; the driver wrapper already selects the target, so drop
that argument in a compiler wrapper.

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
