# Repeated font mappings: native RED to GREEN

Verified 2026-10-17, older x86_64 ABIv11/AROS One pool v11-2 under QEMU,
GCC10.5/AROS-sdk selected by aros-env.toml, Rust1.100.0-nightly
0dfb098f3 (2026-08-31), private aros-net-20261016, unwind build. Shared
SDK/toolchain installations and Photocraft sources remain unchanged.

The exact memmap2 0.9.11/read-fonts 0.41.0 dependency path comes from audited
Fontique 0.11.1. Caller-supplied JetBrainsMono-Regular.ttf is270224 bytes,
1754 glyphs, with its OFL license privately staged. Font/license SHA, source
maps, locked dependency graph identity and build commands are in build records.
The parser checks the same FontRef::from_index/name-table route; no silent
Fontique .ok()? conversion hides its error. This is not a full Fontique build.

Native red02: sysconf selector8 returns-1. Mapping and parsing initially match
fs::read, but repeated map/drop fails at iteration64 (the65th mapping) with
ENOMEM. The table has64 entries; incorrect page size makes memmap2 reconstruct
an invalid base during Drop, so munmap fails and entries remain occupied.

Native green01: Rust uses SDK selector54 and a private rs_sysconf wrapper.
The wrapper returns the actual copy-map allocation granule4096 for SDK page-size
selectors, delegating every other selector to the existing SDK. No SDK symbol
is replaced globally and the64-map capacity is not increased. This prevents
the base-address reconstruction failure instead of concealing it with a larger
registry or a font-loader fallback.

The same diagnostic now passes128 repeated map/parse/drop cycles, exact byte
comparison, name/glyph parsing, unchanged file cursor, unaligned offset3/length31,
64 simultaneous mappings, deliberate ENOMEM at capacity, complete release/reuse,
and retained mapped bytes after the file is closed. Exit status0; RED exits1.
Both runs were staged/collected while stopped and selected collected report
hashes verified against their controller manifests. Fresh report names/compiled
UUIDs distinguish attempts. Screenshots and licensed font binaries remain local.

## Limits

This confirms a read-copy mapping lifecycle bug and its fix on this named
profile. It does not establish whole-Photocraft font cache/selection/render
correctness or Tokio-net readiness. Photocraft needs its own private rebuild
and GUI/font diagnostics. The adapter still has a64-live-map limit, no writable
shared file mapping/protection acceptance, and no concurrent pread acceptance
for users sharing an open file's mutable position. No hardware/v1/arm64 result.
The common integration gate reruns other native consumers after this libc change;
its report is separate from this matched minimal reproduction.
