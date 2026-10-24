# Console geometry from Rust

The `TIOCGWINSZ` compatibility wrapper resolves the requested descriptor through
`__get_default_file`, including when called from a worker thread. It queries
that handle, not the calling task's `Input()` or `Output()`.

1. Try `ACTION_CONSOLE_GETGEOMETRY`, version 1: copied rows, columns and the
   pixel area occupied by complete cells. This matches `dos/consoleinfo.h` in
   [AROS PR #1763](https://github.com/aros-development-team/AROS/pull/1763), head
   `4fd9618699101fa6425146d7d0c0eb7767fe23cf`. It is a proposed upstream contract,
   not a claim that released AROS libraries implement it. Fixed-width local
   structures allow compilation with older SDKs.
2. Only `DOSFALSE` with `ERROR_ACTION_NOT_KNOWN` permits the fallback:
   `XTY_GET_PRESENTATION` from the frozen Limpet v1 packet protocol. Check its
   reply magic, version, operation and known-geometry bit. This preserves
   cell dimensions on older released Limpet versions. Pixels are unknown
   there and are returned as zero.
3. Unsupported consoles return `-1`/`ENOTTY`. In particular, the wrapper no
   longer guesses that `ACTION_DISK_INFO.id_InUse` points to an `IOStdReq`.
   Some handlers use a count in that field. Old CON handlers without either
   copied-snapshot query therefore return `ENOTTY`, rather than going through
   the unsafe pointer fallback.

Malformed replies do not trigger fallback. Zero cells are invalid; dimensions
that exceed `unsigned short` produce `EOVERFLOW`. Unknown presentation bits are
ignored. Every failure leaves the caller's `winsize` untouched. Null output
produces `EFAULT`; an invalid descriptor produces `EBADF`.

## Verification, 2026-10-24

The host test extracts the production function unchanged and supplies the DOS
boundary. It exercises public success, malformed sizes/versions, zero cells,
overflow in all four dimensions, fallback after a failure that contaminates the
output buffer, an unsupported console, malformed presentation replies, unknown
bits, null output, invalid descriptors and non-interactive handles. It runs
under ASan/UBSan. The old function fails the public-query case. An intermediate
version failed the malformed-positive-reply case; the final version rejects it
without fallback.

Run `python3 -m unittest discover -s tests -p 'test_*.py'`. The full suite has
106 tests; localhost socket permissions are needed by existing transport tests.

The full `compat.c` compiles for x86_64 ABIv11 using `AROS/toolchain` and
`AROS/sdk`, and for x86_64 ABIv1 using `/Volumes/arosmain/toolchain-mainline`
and `/Volumes/arosmain/build/bin/pc-x86_64/AROS/Developer`. The mainline build
has existing `posix_memalign` declaration warnings, also reproduced on the
unchanged base. The geometry-only native probe builds with `-Werror` on both.
This is not a rebuild of Rust std or a qualification of Helix/eza binaries.

`tests/build-console-geometry.py OUTPUT` extracts the same production function
and builds `tests/native/console_geometry.c` against real AROS headers. Set
`AROS_GCC` and `AROS_SDK` to a matching pair. Run the probe with stdout redirected
to a log while stdin remains the console. It checks the main task and a pthread,
as well as invalid descriptors, null output and `NIL:`. `ENOTTY` is an expected
safe result for unsupported consoles; the log prints which result occurred.
Native runtime results belong to the parent integration report; this branch's
build result alone does not establish runtime support on either ABI.
