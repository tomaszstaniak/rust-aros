# Common AROS acceptance

This builds real C, no_std and std executables for two explicit CPU/ABI/SDK
profiles. Both profiles can use the same private Rust 1.100.0-nightly source
sysroot; ARM reuses Jonx's PAL rather than reconstructing it. No shared SDK,
toolchain, rustup registration or guest is changed by these build tools.

## Rebuild

Install `nightly-2026-09-01` with `rust-src` (its compiler commit is dated
2026-08-31; the distribution date is one day later). Clone
<https://github.com/jonx/rust-aros> and check out
`2e2845efb6aaa111b5a26c9685e96d08fad340f4`; clone
<https://github.com/pomaros/pomaros-hosted> and check out
`3bc9359d439f9fb5655cf48924413ffe670931cc`. Keep both checkouts clean.
Their source licenses and authorship remain with those repositories.

```sh
python3 tools/common-sysroot.py /path/to/jonx-rust /new/private/sysroot
# An already installed alias is permitted only if its full version matches:
# append --base nightly (the session used this with 0dfb098f3).

python3 tools/build-acceptance.py --profile aarch64-v1-raspi \
  --sysroot /new/private/sysroot \
  --cc /native-pi-toolchain/bin/aarch64-aros-clang \
  --ar /native-pi-toolchain/bin/aarch64-aros-ar \
  --nm /native-pi-toolchain/bin/aarch64-aros-nm \
  --sdk /native-pi-toolchain/sdk --glue /path/to/pomaros-hosted \
  --out /new/private/build-arm

python3 tools/build-acceptance.py --profile x86_64-v11 \
  --sysroot /new/private/sysroot \
  --cc /v11-toolchain/x86_64-aros-gcc \
  --ar /v11-toolchain/x86_64-aros-ar --nm /v11-toolchain/x86_64-aros-nm \
  --sdk /matching-v11-sdk --out /new/private/build-x86
```

The session's ARM pair is native AROS `20260822` Developer SDK / Clang 20.1.0,
with Cortex-A53 and conservative x18 reservation matching its C wrapper.
This reservation is not asserted to be a native AROS requirement. x86_64 uses
the older AROS One v11 SDK / GCC 10.5.0. Host and target compilation always use
the explicitly selected Rust wrapper, including Cargo build scripts.

The ARM SDK contract compilation checks time/pthread widths, offsets and
synchronization buffer alignment/capacity against its actual headers. File
metadata crosses Jonx's C bridge; it does not reuse v11 stat bindings.
Build manifests record source/SDK hashes and exact compiler versions.

## Run with a deadline and actual exit receipts

```sh
python3 tools/package-acceptance.py /new/private/build-arm /new/private/bundle \
  --programs RAM:rustprobe --results RESULTS: --run-id unique-run-id
```

This only creates a directory. Determine the exact physical Pi model, matching
AROS image and dedicated transfer destination before deploying. Do not flash
media or replace system libraries as part of this recipe. Follow the testbench
reservation/stopped staging procedure for QEMU.

Copy the bundle into the specified private guest drawer and ensure the result
drawer exists and is writable. Inspect the transfer log and artifact hashes.
Run `Execute RAM:rustprobe/run` from a fresh Shell. An Execute dialog may retain
the previous command: clear and inspect it before typing.

Set a host-side **45-second total deadline before starting the command**. The
session's QEMU observer stops its own reserved machine at that deadline, then
reads the stopped result medium. On hardware, record the deadline, wait for
the final receipt, and stop the observation at the deadline; a hung process
does not pass. Do not reset a shared physical machine without authorization.

The sequence is C, no_std, then three std runs. Each command writes its output
to a fresh log followed immediately by `Echo $RC` into its own `.rc` file.
Require every receipt to contain exactly an integer zero, all expected work
markers, and the final `complete` receipt. A work-complete message alone is
not success. Missing receipts, aborted/panic exits and timeout are failures.
Retain the build manifest, transferred artifact hashes, observer timestamps,
logs and all five real exit receipts together. Do not use a stale result name.

The std probe covers aligned Vec growth, BTreeMap/HashMap, file write/read/stat/
remove, advancing Instant, SystemTime read, four workers, mutex counter=4000,
four channel messages, and all four joins. Wall-clock calibration is separate.

Native sockets are deliberately unavailable until an owner-task backend is
integrated. No socket Send/Sync workaround is added. Cryptographic entropy
quality, ARM unwind, async/TLS, GUI applications and target cargo-test execution
are separate gates. `cargo test --manifest-path acceptance/Cargo.toml` exercises
the workload on the host only; it does not count as AROS execution.

Physical Pi acceptance remains pending even when native AROS passes under QEMU.

## Separate ordinary Cargo executable gate

The staticlib gate above does not exercise Rust's `lang_start`. After creating
an acceptance build and a current guarded common sysroot, build the separate
Rust binary (no C `main` is linked):

```
python3 tools/build-cargo-main.py BUILD_DIRECTORY COMMON_SYSROOT NEW_MAIN_BUILD
```

The command checks compiler/source/SDK identity, uses the same private compiler
for host and target, and retains the target profile in the new output. ARM uses
the seven C bridge objects retained by the acceptance build. The final link
keeps AROS CRT symbol-set sections; it explicitly verifies ELF ABI, a defined
`main`, and no undefined symbols. This gate allows the platform C driver to
supply its native CRT/default libraries. The private linker adapter appends
`--no-gc-sections` after Cargo's arguments. Using `-Clink-dead-code` for this
purpose would also compile unused architecture intrinsics while rebuilding core.

Copy `NEW_MAIN_BUILD/cargo-main` into a private guest drawer while the reserved
VM is stopped. Run three fresh instances with `runtime-argv workload`, then one
with `runtime-argv exit7`. Set a 4 MiB stack and a 45-second absolute host
deadline. Redirect each invocation to its own new log and immediately write
`Echo $RC` to a new receipt. Require receipts `0, 0, 0, 7`, the local-destructor
marker in every log, and all workload markers in the first three. A pre-exit
message alone is insufficient. Keep these receipts separate from staticlib
acceptance. Runtime acceptance is recorded in external evidence, never inferred
from the builder's `runtime_acceptance: false` manifest.
