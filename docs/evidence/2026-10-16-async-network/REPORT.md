# Async networking and TLS preparation, 2026-10-16

Status: in progress; no new guest runtime acceptance.

## Target and backend decision

The active target is x86_64 ABIv11, GCC 10.5.0, the older AROS/sdk profile,
and private Rust 1.100.0-nightly (0dfb098f3 2026-08-31). v1 and arm64/Macaros
remain deferred. The v11 shared SDK OpenSSL 1.1.0h is not selected for HTTPS.
Use Mbed TLS 3.6.7 and jitterentropy 3.7.0 from apkg. A newer OpenSSL port
from v1 is an alternative only after separately compiling and bundling it for
v11; v1 libraries or runtime results are not interchangeable with v11.

## Exact dependency compile experiment

`async-inputs.json` records the audited Zed revision, lock digest and checksums
of the published polling/async-io/async-net/smol inputs. The disposable consumer
uses async-net 2.0.0, async-io 2.6.0, polling 3.11.0 and futures-lite 2.6.1.
The first resolution selected rustix 1.1.5. Repeating with rustix 1.1.4, the
version in the audited Zed lock, also fails with 395 Rust diagnostics.
This is a bounded compile result, not a claim that the complete consumer graph
matches Zed: other transitive dependencies have not yet been pinned to its lock.

The first failures include missing flock constants, dynamic-loader symbols,
poll flags, process APIs and differing pid/sockaddr_un assumptions. The selected
rustix libc backend needs explicit AROS adaptation; changing a target cfg alone
has not enabled the consumer. Do not add invented constants to silence it.
Investigate a bounded polling/async-io AROS backend instead of assuming all of
rustix must be ported for the first TCP consumer.

Retained local exact-version log:
`AROS/local/rust-async/consumer-initial/build-rustix-1.1.4.log`, SHA-256
`b251fb7b3fd1bd6aae37fc6fba78e6375fe4bf3200c012b9015a9f45f30e2738`.

## Private TLS build/link experiment

Readonly source: apkg revision `91ac4d8f00bbaf5ac5b057322d3937a61acd1cd7`.
Copied inputs, source manifest, build experiment and outputs are under
`AROS/local/rust-async/tls-initial` and `tls-build-initial.py`.
`tls-build.json` records individual copied source and output hashes. No shared
SDK, library, sysroot or apkg source was changed. Existing apkg packaged SDK
artifacts were not reused.

The experiment consumes apkg's `sdk/mbedtls-sources.txt` and preserves its
Mbed TLS -O2/gnu99/config/test-inet flags and jitterentropy -O0/-fwrapv recipe.
The ordinary and AROS_JENT_FORCE_FAIL entropy adapters compile separately.
All seven static archives build. The unmodified apkg SDK `https_get.c` example
links twice, once per entropy adapter, with Mbed TLS/X509/crypto/platform/jent
link order. These are build/link results only. The copied blocking example is
not the proposed async TLS adapter or a new HTTPS runtime acceptance.

Compiler output contains endian-macro redefinition warnings from the SDK/header
combination. Both example links exit successfully. Preserve the logs for review;
these experiments do not establish header/SDK isolation beyond the named inputs.

## Remaining gates

- Run the initial std TCP socket/nonblocking/readiness/drop probe before fixing
  descriptor routing. Current namespace/owner-task concerns are hypotheses backed
  by source inspection, not new guest measurements.
- Verify selector ownership and wakeups, then accept the real async consumer.
- Add stepped opaque TLS operations that return WANT_READ/WANT_WRITE rather
  than retrying blindly. Preserve required chain, name and date verification.
- Run controlled valid/wrong-name/expired/untrusted/entropy-failure cases.
- Run existing libc/process/LSP regressions after any boundary change.

At the last availability check all three ready v11 slots belonged to other
sessions. No lease was acquired and no guest was touched. Do not use v1 or the
unprovisioned scratch slot as a silent fallback.
