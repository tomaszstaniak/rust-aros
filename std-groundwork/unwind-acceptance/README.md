# x86_64 ABIv11 unwind acceptance

A bounded test of Rust stack unwinding, using the existing GCC 10.5 unwinder.
It checks typed payloads, destructor execution, catch/rethrow, four joined
threads with 16 caught exceptions each, and a worker panic returned through
join. Exactly 67 guards must be destroyed.

Host control, from the repository root:

    cargo +nightly run --manifest-path std-groundwork/unwind-acceptance/Cargo.toml --release

Native build, from this directory, with an isolated AROS std toolchain:

    CARGO_TARGET_DIR=YOUR_PRIVATE_UNWIND_OUTPUT cargo +YOUR_AROS_TOOLCHAIN build --release --offline

The package-local config explicitly selects panic=unwind, force-unwind-tables
and build-std std/panic_unwind. The generated global target's abort default
and shared toolchains remain unchanged. Do not mix this build directory with
previous abort-profile output.

The test intentionally uses resume_unwind for most cases to match Salsa's
cancellation mechanism. The last case uses panic_any with a temporarily quiet
hook and verifies the failed join payload and destructor; the hook is restored.

Use ports/wat-server/stage.py --unwind-probe PATH alongside the real server,
client and --cancellation-probe PATH. The independent unwind probe runs first
and writes separate output/rc, followed by normal LSP and deterministic Salsa
cancellation. All guest binaries run from RAM:wl with explicit execute
permission. Use a reserved matched v11 machine and collect only while stopped.

These checks do not establish C++ exception interoperability, unwinding through
arbitrary C ABI frames, async cancellation, double-panic recovery, other SDKs,
v1 or arm64/Macaros behavior.
