# terminal_size 0.4.3 on x86_64 AROS ABIv1

Apply the series to the registry archive identified in the patch header.
Select the patched crate and the rust-aros dual libc through Cargo
overrides. The AROS build uses the existing `TIOCGWINSZ` adapter and
returns `None` when the descriptor or console handler has no geometry.
Other targets use their upstream dependencies.

Build `probe.rs` with an exact terminal_size 0.4.3 dependency. On AROS,
run from an interactive Shell. The probe checks that `NIL:` returns
`None`, accepts an unavailable geometry response from the console, and
prints `ABIV1_TERMINAL_SIZE_PROBE_OK`.

Verified with Rust nightly 1.100.0 (0dfb098f3), GCC 10.5.0 and the official
x86_64 ABIv1 SDK from 2026-11-10. The unpatched focused build failed in
rustix 1.1.4. The patched probe built and returned 0 on the 2026-11-13
ABIv1 runtime. The stock Shell returned no geometry. Positive geometry
delivery remains untested on this runtime. The patched crate also passed
an offline host Cargo check on macOS.
