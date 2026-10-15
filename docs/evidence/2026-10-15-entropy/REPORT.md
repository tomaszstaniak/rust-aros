# x86_64 v11 entropy and libc acceptance

Verified 2026-10-15, implementation based on rust-aros db458c2 plus this
commit's entropy changes. This is a bounded functional acceptance, not a
cryptographic certification or a whole-platform compatibility claim.

## Build and execution address

- Target: x86_64 AROS ABIv11, older AROS One 1.3 UP installation, pool v11-1.
- Compiler: AROS/toolchain/x86_64-aros-gcc, GCC 10.5.0; matching older AROS/sdk.
  Driver SHA-256 c2783f9b3594f7ccab0ebf17f025ba715db958c7079ddd84f8fceed6abac649a.
  The v11 2026.09 SDK/guest was not used.
- Rust: 1.100.0-nightly (0dfb098f3 2026-08-31).
- std rebuilt into the new local aros-jent-20261015 toolchain from this code.
  The shared aros-nightly/sysroot-8d35fda was not replaced.
- Guest: QEMU x86_64 pc/TCG, qemu64, 2 configured vCPUs (guest Exec is the older
  UP build), 2 GiB, 1024x768; private pool input/results devices.
- Exclusive owner codex-rust-renewal-20261015. Three stopped-stage-start runs;
  each stopped, collected and released afterward. Input extracted into RAM:.
  No guest system libraries, startup files or preferences changed.

## Source and implementation

The unchanged jitterentropy 3.7.0 sources, platform header and serialized
collector adapter were copied from arospkg 91ac4d8f00bbaf5ac5b057322d3937a61acd1cd7.
See libc-aros/csrc/vendor/README.md and bundled licences. Core C sources use
-O0 -std=gnu99 -fwrapv; adapters use -O2. Byte comparisons against the source
checkout passed. Internal timer threads are not enabled.

The POSIX adapter accepts at most 256 bytes, returns EIO for oversized requests
or source failures, EFAULT for nonempty NULL buffers, and success for length 0.
It checks the returned byte count and wipes failed output. The copied collector
is semaphore-serialized and retains source initialization failure. There is no
clock/random fallback. Cleanup is registered through ADD2EXIT under private
symbol names. The direct Rust errno binding was corrected to ___geterrnoptr,
the symbol confirmed in the selected SDK and already used by the std patch.

## Reproduction

Build from each example's directory, where Cargo reads its target configuration:

```sh
cd std-groundwork/entropy-test
cargo +nightly build --release --offline
```

The installed nightly in this environment was checked against the exact pinned
version above. For std, first create an isolated sysroot from repository root:

```sh
AROS_RUST_BASE=nightly sh std-groundwork/std-patch/make-std-toolchain.sh \
  /absolute/new/sysroot aros-jent-local
cd std-groundwork/entropy-std
cargo +aros-jent-local build --release --offline
```

The run used CARGO_HOME=AROS/local/rust-aros-cargo and separate target directories
under AROS/local/rust-aros-build: entropy-probe and entropy-std. The libc baseline
was built from libc-test into original-libc before the entropy change; it has
no entropy assertions. Native forced-initialization-failure probe:

```sh
AROS_GCC=/path/to/x86_64-aros-gcc AROS_SDK=/matching/v11/sdk \
  sh tests/build-entropy-failure.sh /path/to/libaroscompat.a /path/to/entropy-failure
```

Use the archive produced in entropy-probe's target build output. This compiles
the actual copied adapter with AROS_JENT_FORCE_FAIL. No production fallback or
forced-failure Cargo option is introduced. Results are redirected to distinct
RESULTS: files and collected from the stopped VM.

## Accepted cases

| Case | Evidence | Result |
| --- | --- | --- |
| libc time, sleep, file/stat/read, directory layouts, pthread creation/join, mutex lifecycle | rent-1015-libc.txt and rc | 12 [ok], no [FAIL], rc 0 |
| Real source, four readers entering lazy initialization, joined pthreads, repeated reads, length bound | rent-1015-raw.txt and rc | marker present, rc 0 |
| Forced source initialization failure repeated three times | rent-1015-fail.txt and rc | EIO, output wiped, no fallback; rc 0 |
| Rebuilt std HashMap workers, Instant and sleep | rent-1015-std.txt and rc | rc 0 |
| Upstream getrandom 0.3.4 and rand 0.9.5 with explicit custom backend | rgrf-1015-std.txt and rc | 513-byte fill and fill_uninit, StdRng seed, HashMap/thread checks; rc 0 |

binaries.sha256 describes the final payload, including the final run's updated
entropy-std binary. It does not identify the earlier std-only binary, which was
replaced before its hash was recorded. The second getrandom run was preliminary;
the final run supersedes it after correction of the uninitialized-memory contract. Pool collectors verified file hashes
against their stopped read-only results disks. Full manifests remain under
AROS/local/rust-entropy-results, AROS/local/rust-getrandom-results and
AROS/local/rust-getrandom-final-results.

Host suite: 17 tests pass. New contract tests inject poll failure/short return,
verify length and NULL handling, and compile the production getrandom callback
against an injected libc source to check 256/256/1 chunking and full-output
clearing after a mid-request failure, including a MaybeUninit input. The callback
uses raw pointers, avoiding a byte-slice reference over uninitialized memory.
Miri is not installed; the host test is not a substitute for a Miri run. Existing build tests check the actual
compiler argument lists, including -O0 for every jitter core source.

## Crate integration and host findings

The initial unmodified getrandom 0.3.4 build failed with its unsupported-target
error. The example uses its documented custom-backend symbol and cfg; rand
0.9.5 uses that instance. No upstream getrandom/rand source was patched.
This is an explicit application integration, not automatic upstream support.
getrandom 0.2 and 0.4 were not validated.

The host emitted a nonfatal warning while stripping a build-script binary:
rust-objcopy could not load libLLVM.dylib from the installed nightly's host-tool
layout. Target builds completed. This host-tool packaging issue remains open;
no shared Rust installation was modified to hide it.

## Upstream review

Read through GitHub API on 2026-10-15, both default branches named master:

- deadwood2/AROS 5b5fd4cfd4f039927c0d1bb9cdd7f4fc8644d23e (2026-10-02).
  Last five CRT changes include locale_t header selection (4bd27247416e),
  gcvt and readlink fixes. A lookup of compiler/crt/posixc/getentropy.c returned
  404; this is only absence at that path, not a whole-tree capability claim.
- aros-development-team/AROS 38ba2794c1815c84f5f8d6303aacaec04af6ecf2 (2026-10-15).
  Current getentropy.c blob e99eb21b615fc17d3250ec6a1c2842e4e477a4f4 uses
  entropy.resource through optional-library handling. Its oversized/NULL/zero
  argument handling differs from this adapter; a v1 profile must use and test
  the actual system contract, rather than importing the v11 definition blindly.
  Recent CRT changes include feature-request handling, plain-char limits and
  pread/pread64. They do not establish presence in any local guest or SDK.

The initial web-tool API requests failed; the authenticated read-only gh API
requests succeeded with host network permission. Default-branch inspection is
not a full release-channel audit. Mainline/Macaros source selection remains a
separate acceptance task.

## What this did not establish

No fresh v11 2026.09, mainline v1, hosted arm64 Macaros, SMP, hardware, KVM,
load/snapshot or SP 800-90B assessment. Different buffers are a smoke check,
not a statistical entropy measurement. Source health/read failure is injected
at the host POSIX boundary; native injection exercises initialization failure,
not a real hardware health failure. Normal exit succeeds, but no counter proves
the ADD2EXIT hook freed the collector. No HTTPS/TLS validation was performed in
this Rust run. These are follow-ups, not reasons to bypass source failures.
