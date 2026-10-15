# Rust Limpet presentation acceptance

Verified 2026-10-15. Rust-aros base 4e74415 plus the aros-limpet crate in this
commit. Source snapshot: Limpet SDK 0.3.0, tag sdk-v0.3.0, commit
6fa16a87fcdcc75f0790e967242f5485fb1e4ffc; vendored files match git objects.

## Address and builds

Target x86_64 AROS ABIv11, older AROS One 1.3 UP pool v11-1, QEMU pc/TCG,
qemu64, 2 configured vCPUs, 2 GiB, 1024x768. AROS/toolchain GCC 10.5 and matching
older AROS/sdk; not the v11 2026.09 acceptance profile. Rust 1.100.0-nightly
(0dfb098f3 2026-08-31). no_std uses the installed pinned-version nightly;
std uses the isolated aros-jent-20261015 sysroot. Shared sysroots unchanged.

From the repository root, run setup.sh with the explicit v11 compiler/SDK/AR.
Build from examples/limpet with cargo +nightly build --release --offline, and
from std-groundwork/limpet-std with cargo +aros-jent-20261015 build --release
--offline. In this run the installed nightly was checked against the pinned
version. CARGO_HOME was AROS/local/rust-aros-cargo; target directories were
AROS/local/rust-aros-build/limpet-native and limpet-std.

Limpet was the published 0.4.1 binary, from Limpet-0.4.1-abiv11.zip, SHA-256
041e0886c546a4fb61f7b63c318b70b1e83c8f740e02a741305826be2dd985a4. Its source tag
v0.4.1 resolves to 80398c5bb1f441b8e526d830553054da2b646c9c. No terminal rebuild,
shared installation change or claim about a newer release. Fonts/themes came
from that same verified archive. Input was unpacked into RAM:rl.

Exclusive reservation codex-rust-renewal-20261015. Stop-stage-start, file-based
results, normal EndCLI of the private Limpet session, stopped collection and
release. No guest libraries, preferences, startup files or installed programs
were changed. Full verified collection manifest: AROS/local/rust-limpet-results.
The guest-created report files below were copied from that collection.

## Cases

| Case | no_std | std |
| --- | --- | --- |
| Stock CON: | Unsupported, no geometry/rewrite, IoErr 209, rc 0 | same, rc 0 |
| File redirected from CON: | NotInteractive, no geometry/rewrite, rc 0 | same, rc 0 |
| Limpet 0.4.1 Shell | Supported, 80 columns/24 rows, rewrite true, IoErr 0, rc 0 | same, rc 0 |
| File redirected from Limpet | NotInteractive, no geometry/rewrite, rc 0 | same, rc 0 |

All eight exit codes are zero. The four redirected output files contain no ESC
bytes; query uses Output(), so Input() remaining interactive does not enable
colour/title/progress controls in a file. Both scripts wrote completion markers.

con.png shows plain text and progress lines. limpet.png shows green/cyan ANSI
colours, readable Polish UTF-8 and the final progress line. title.png shows the
explicit Rust title while the extra no_std probe runs; shell-return.png shows
the ordinary Shell title and command prompt restored afterward. The two extra
title probes have query reports and screenshots but no separately recorded rc.
Screenshots establish final layouts, not animation timing or tear-free output.

binaries.sha256 names the exact payload binaries/scripts used by the guest.
After staging, the constructor was made unsafe to state foreign-handle lifetime
and task exclusivity requirements explicitly, and Rust files were formatted.
Final examples were rebuilt successfully; those source/API-only changes were
not restaged for another guest run. There was no transport/formatting behaviour
change between the tested binaries and final rebuild.

## Host checks and API contract

Six Rust host tests exercise the actual pinned SDK formatting/decoding code:
known/unknown fields, Workbench pens versus ANSI colours, RGB/indexed fallback,
title length and UTF-8/control/NUL rejection, malformed replies, geometry, line
controls, fallback and write failure. Transport injection checks every failure
status and an unknown status, clearing capabilities before decoration.
The existing 17 Python host tests also pass.

The AROS glue compiles the same SDK query and short-write implementation. It
keeps BPTR/LONG conversions in C and asserts the presentation layout. Output is
not Send/Sync; unsafe Output::current requires the caller to keep the process
output handle valid/unchanged and exclude concurrent access from other tasks.
That requirement is not silently solved by the task-local marker. Pure
Presentation/Sequence operations need no unsafe call and allocate no memory.

A nonfatal host rust-objcopy/libLLVM load warning recurred while stripping a
build script in the isolated std build. Build/link succeeded. The existing
host-tool packaging follow-up remains open; no shared compiler was modified.

## What this did not establish

No raw keyboard input, resize subscription, PTY/termios, Ratatui, async I/O,
hyperlinks or image output. No native query/write failure injection, short-write
stress, concurrent output, v11 2026.09 guest, mainline v1 or arm64/Macaros run.
The host fake transport does not prove C query/short-write failure paths; the
native matrix covers their ordinary supported/unsupported/noninteractive paths.
SDK queries are synchronous and DOS provides no timeout. Buffered std/C output
must be flushed by callers before capturing/querying the DOS output.
