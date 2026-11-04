# Explicit getrandom 0.3 backend on x86_64 AROS v11

This example uses the new std sysroot and the real jitterentropy-backed libc
adapter. It tests std HashMap seeding on four joined workers and explicitly
integrates getrandom 0.3.4 (fill and fill_uninit) and rand 0.9.5
(StdRng::try_from_os_rng).

Upstream getrandom 0.3.4 does not select an AROS backend. The first unmodified
build failed with its unsupported-target error. This example uses its supported
custom-backend interface: the Cargo config selects getrandom_backend="custom",
and src/aros_random.rs exports __getrandom_v03_custom exactly once. Requests are
split into at most 256-byte libc getentropy calls. On any failure the callback
returns an error and clears the entire requested output. No weak fallback.

These crates are not forked. The application must opt into and supply the
backend; this does not enable arbitrary applications or getrandom 0.2/0.4.
The callback's custom error code 1 means its underlying getentropy source failed.
The example is specific to the tested x86_64/v11 profile; do not enable this cfg
for host build scripts or Macaros without checking their actual target setup.

From the repository root, create a new sysroot using the std-patch installer,
then build from this directory with cargo +YOUR_NEW_TOOLCHAIN build --release.
A pre-existing shared aros-nightly is not refreshed by editing libc-aros.

Native result and limits: ../../docs/evidence/2026-10-15-entropy/REPORT.md.
