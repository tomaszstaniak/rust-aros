# Review corrections, 2026-10-22

The shared base remains rustc 1.100.0-nightly (0dfb098f3, distribution
nightly-2026-09-01), with Jonx PAL 2e2845efb6aaa111b5a26c9685e96d08fad340f4.
No shared SDK, compiler, upstream checkout or prior acceptance evidence changed.

## Thread ownership and TLS

The ARM-only no-op guard bypassed `thread/current.rs::set_current`'s contract:
`CURRENT` owns the reference produced by `Thread::into_raw`; its key has no
own destructor. Preserve the compiler's `sys/thread_local/guard/key.rs` instead.
On this target `target_thread_local` is false. That guard uses pthread-key
DEFER/RUN passes. Each `os.rs::destroy_value` re-arms DEFER after dropping its
value; only a subsequent pass without Rust destructors calls
`rt::thread_cleanup` -> `drop_current`. The latter marks CURRENT destroyed
before reconstructing and dropping its owning reference. No cleanup was added
to Jonx's thread entry before pthread destructors.

The local AROS pthread source (`/Volumes/macaros/aros-upstream/compiler/pthread/
pthread_create.c`, lines 114-131) clears a key before invoking its destructor,
repeats destructor rounds and signals join only afterwards. This source is
supporting contract evidence; the native runtime checks below verify the
installed systems separately.

`acceptance/src/lifecycle.rs` observes the Thread-owned CString allocation
through a diagnostic global allocator. For each of 16 named workers, join must
leave exactly one name while an explicit Thread handle exists; dropping that
handle must restore the allocation count. The check also requires exactly 32
TLS drops, checks `thread::current()` in each destructor and initializes a second
TLS value from the first destructor. Its keys are created before destruction
because AROS holds a shared TLS-key semaphore while invoking callbacks.

Both staticlib workloads passed three times with actual shell RC=0:

- AArch64, native AROS ABIv1 20260822 SDK, Cortex-A53, rpi-1 QEMU raspi3b;
  Clang 20.1 AROS wrapper, +reserve-x18 / -ffixed-x18.
- x86_64, ABIv11 SDK, GCC 10.5, v11-1 AROS One QEMU.

Each run reported names balanced=16 and TLS drops=32, then collections, file,
time, mutex/channel and joined-worker checks. Each observation had a 45-second
host deadline. The pre-fix ARM private sysroot failed to produce a std RC;
a second reproduction captured its software-failure requester. This is failure,
not runtime acceptance. The host workload and all 94 Python tests passed.

Evidence: `local/rust-review-fixes-20261022/`:
`arm-{red,fixed}-build.log`, `arm-{red,fixed}-results/`, `x86-fixed-build.log`,
`evidence/x86-staticlib/`, deadline JSON files and `red2-assertion.png`.

## Sysroot and package contracts

The common sysroot manifest now has an exact versioned schema, compiler/PAL,
profile set, adaptation input hashes and complete library/compiler-source tree
hashes. Both CPU builders validate it before invoking a compiler or creating
an output directory. Old markers require regeneration. This detects accidental
stale or modified trees; it is a provenance receipt, not a signed security
boundary against someone rewriting both the sources and their recorded hashes.

Packaging requires exactly C/no_std/std SHA-256 entries and copies the bytes it
verified. Invalid input creates no bundle; a later write failure removes the
newly owned incomplete bundle. Existing destinations are preserved.

## Limits and next gate

These results still exercise a C CRT main plus Rust staticlib. A separate Cargo
binary with Rust `fn main()`, argument initialization and real return-code
receipts is required before claiming ordinary Rust executable acceptance.
No physical Pi, sockets, GUI or ARM unwind acceptance is claimed.

## Ordinary Cargo main acceptance

The separate gate subsequently passed on both CPU/runtime combinations above,
using the new private `rust-review-sysroot-main-20261022`. Its ARM args adaptation
is versioned in `patches/jonx/0002-native-lang-start-args.patch`: PAL init captures
argc/argv using atomics before the Rust main runs. It removes dependence on C
harness globals. x86_64 retains its existing argument backend.

`tools/build-cargo-main.py` builds `acceptance/src/main.rs` with Cargo, using
Rust's generated main and lang_start, the platform C CRT, and the existing ARM
C library bridges. No C main object is linked. It verifies the current guarded
sysroot, compiler, SDK digest, ELF ABI, defined main and absence of undefined
symbols. The private linker adapter retains AROS CRT sections at final link.
Without this, ARM ELF section GC discarded main and produced a misleading 14 KiB
executable; the checked executable is approximately 1.9 MiB with main/lang_start.

Both `rpi-1` and `v11-1` produced fresh shell exit receipts **0, 0, 0, 7** within
the 45-second host deadlines. The first three executions each repeated the full
std workload, 16 balanced Thread-name allocations and 32 TLS destructor calls.
The fourth verifies nonzero `ExitCode` propagation. All four logs contain the
local-main-destructor marker. Argument count and sentinel values are asserted
inside the Rust main. Host Cargo also returned 7 with the destructor marker;
all **95 Python tests** and the host Rust workload passed.

New evidence and executable bundles are in `arm-main-results/`,
`evidence/x86-main/`, `{arm,x86}-main-final/`, and `{arm,x86}-main-bundle/` beneath
the same private evidence root. Old staticlib evidence is preserved separately.
This closes the ordinary Rust executable gate on the named QEMU installations;
it does not establish physical hardware, socket, GUI or ARM-unwind acceptance.

## Final review and retention

A fresh read-only review of 87c03fd..ea1f66c reported no concrete P1/P2
findings. The five local correction commits are b11a8d4 (package inventory),
ce51a02 (sysroot contract), fcada80 (existing diagnostic compatibility),
2f097cc (TLS ownership), and ea1f66c (ordinary Cargo main). Nothing was pushed
or merged; the integration branch has no configured upstream.

The evidence directory contains `runtime-summary.json`, selected guest receipts
verified against the pool collector manifests, source contracts and build
manifests. All session Cargo caches and duplicate full RESULTS collections were
removed after preservation: 1,395,941,797 bytes reclaimed. Private source
sysroots (approximately 79 MiB each) are retained for reproduction and review;
they do not replace any installed sysroot. Both reserved VMs were stopped and
released. Original pre-review evidence and the original checkout's modified
`.DS_Store` were untouched.
