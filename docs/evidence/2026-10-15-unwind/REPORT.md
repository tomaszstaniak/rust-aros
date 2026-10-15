# Explicit x86_64/v11 unwind profile, 2026-10-15

## Scope and isolated change

Older AROS One 1.3 UP, pool v11-1, QEMU pc/TCG qemu64, two configured vCPUs,
2 GiB; GCC 10.5 and AROS/sdk, Rust 1.100.0-nightly
(0dfb098f3 2026-08-31), base isolated toolchain aros-exec-20261015.
v1 and arm64/Macaros remain deferred.

The opt-in profile compiles std with panic_unwind and application/std flags
-Cpanic=unwind -Cforce-unwind-tables=yes, using fresh private target directories.
It uses the installed v11 libgcc without rebuilding/replacing it. Sources,
global target JSON (still panic-strategy=abort), shared sysroots, SDK and guest
libraries remain unchanged. profile.json records flags, empty ambient override
variables and libgcc hash; unwinder-symbols.txt records the actual linked
unwinder, Rust personality, frame registration and dwarf register-size table.

The prior C++ exception diagnosis in aros-boxerapp demonstrated that GCC 10.5
v11 already supplied a functioning unwinder; the separately diagnosed broken
mainline/GCC13 configurations were not imported into this build.

## Red baseline and native result

The prior [deterministic Salsa probe](../2026-10-15-wat-concurrency/REPORT.md)
opened real WAT and passed ordinary symbol analysis, then aborted with rc20
after cancelling the snapshot's token under panic=abort. Host unwinding
caught the cancellation. The new profile changes code generation/panic runtime;
the same Salsa/LanguageService cancellation source remains unchanged.

| Native case | Result |
| --- | --- |
| resume_unwind typed payload and destructor | PASS |
| catch, rethrow, typed payload and destructor | PASS |
| four worker threads, 16 caught exceptions each, joins and cleanup | PASS |
| uncaught worker panic, join error payload and destructor | PASS |
| exactly 67 destructors across the independent probe | PASS, rc0 |
| real WAT diagnostics, Unicode repair, formatting and symbols | PASS |
| 16 outstanding mixed LSP requests, shutdown, EOF and stderr join | PASS, client/server0, stderr0 |
| real Salsa query after deterministic token cancellation | caught, None, rc0 |

Raw evidence: uw01-unwind.txt/.rc, uw01-transcript.txt, uw01-rc.txt,
uw01-cancellation.txt/.rc and final uw01-done.txt. No abort requester occurred.
The host control and 23 Python/two client regressions also pass; logs retained.
An ordinary abort-profile server build was re-run successfully in its separate
original output directory. Ambient RUSTFLAGS and CARGO_ENCODED_RUSTFLAGS
rejection checks both pass before the build can proceed.

## Reproduction

From std-groundwork/unwind-acceptance:

    CARGO_TARGET_DIR=PRIVATE_OUTPUT cargo +AROS_TOOLCHAIN build --release --offline

That package config selects the explicit unwind flags and std/panic_unwind.
Host control runs from repository root with cargo +nightly run --manifest-path
std-groundwork/unwind-acceptance/Cargo.toml --release --offline.

For pinned WAT/Salsa use ports/wat-server/build.py with --aros --unwind,
a private CARGO_TARGET_DIR, and optionally --cancellation-probe. Clear ambient
RUSTFLAGS and CARGO_ENCODED_RUSTFLAGS; the WAT recipe rejects nonempty values
for this profile. The generated global target is not edited.

ports/wat-server/stage.py --unwind-probe PATH and --cancellation-probe PATH
stage the independent unwind probe, actual server/client, and engine probe in
one new payload. Protect +e precedes execution from RAM:wl; reports have unique
RESULTS: names. Exact executed files are identified by payload-sha256.json.
The owned guest was stopped before collection, then released. Full collection
and manifest: AROS/local/rust-unwind/results-01.

## Boundaries and next acceptance

This establishes Rust unwinding and the specific Salsa cancellation path on
the matched x86_64/v11 profile. It does not establish foreign C++ exception
interoperability, arbitrary C ABI unwinding, double-panic recovery, complete LSP
$/cancelRequest delivery/response semantics, document-change overlap, forced
process termination, other v11 SDKs, v1 or arm64/Macaros.

Use the explicit unwind profile for the next real interactive LSP tests:
document changes during active analysis, cancellation and subsequent recovery,
and closing the session while requests remain active. Keep the ordinary abort
profile available for existing consumers.
