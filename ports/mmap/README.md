# Font mapping lifecycle diagnostic

Build with `build.py NEW_OUTPUT --font FONT --font-license LICENSE --toolchain
PRIVATE_TOOLCHAIN --cargo-home PRIVATE_CACHE [--profile unwind|abort]`. It
copies owned sources, uses the selected private libc source for std and crates,
records source/payload hashes and a compiled UUID, and produces a stopped-VM
payload. Extract to RAM:mm, verify extraction, then Execute RAM:mm/run-tests.
Every attempt uses new report names and refuses existing ones.

The fixture is an immutable caller-supplied font. read-fonts0.41.0 is the parser
used by audited Fontique0.11.1; memmap2 is pinned0.9.11. The probe compares
whole-file bytes with fs::read and name/glyph parsing, repeats128 map/drop cycles,
checks an unaligned view, preserves the file cursor, holds64 simultaneous maps,
checks explicit capacity rejection, frees/reuses them, and retains data after
closing the file. It exposes errors rather than silently converting them toNone.

The current AROS adapter is an allocated read-copy implementation. Its4096
alignment granule must reach memmap2 through sysconf(_SC_PAGESIZE), including
the correct SDK selector54. Otherwise memmap2's allocation-base reconstruction
on Drop can fail and exhaust the64-entry table. Private rs_sysconf reports the
mapping granule and delegates other selectors to the selected SDK.

This accepts neither writable shared mappings, page protection, arbitrary
partial munmap nor concurrent pread operations sharing a mutable file position.
It does not prove full Photocraft font selection/cache/render behavior. The font
and license are staged privately rather than vendored into this repository.
