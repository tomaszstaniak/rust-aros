# Combined older x86_64 v11 acceptance

`gate.py` prepares fresh private builds of the pinned socket boundary, real
async-net consumer, Mbed TLS/jitterentropy HTTPS gate, entropy example, five
process cases, WAT LSP suite and exact memmap2/read-fonts diagnostic. It never
installs a shared SDK/sysroot or controls a VM. It uses the existing selected
AROS GCC/SDK settings in aros-env.toml and the named private rustup toolchain.

Create a private JSON configuration with absolute paths:

```json
{
  "profile": "unwind",
  "toolchain": "YOUR_PRIVATE_TOOLCHAIN",
  "cargo_home": "/private/cache/with-pinned-rust-aros-dependencies",
  "mmap_cargo_home": "/private/cache/with-memmap2-and-read-fonts",
  "archives": "/private/checked-async-crate-archives",
  "apkg": "/readonly/apkg-repository",
  "wat": "/readonly/wat-428f74d",
  "font": "/readonly/JetBrainsMono-Regular.ttf",
  "font_license": "/readonly/OFL-JetBrainsMono.txt"
}
```

The font is a caller-supplied immutable fixture, staged with its license, and
its SHA is recorded. The probe pins memmap2 0.9.11/read-fonts 0.41.0 to match
the audited Fontique path. It does not load the full Fontique cache/UI. Cache
copies must be private: Cargo may write its home. Photocraft's source/cache
remain read-only; do not use its live cache as this command's Cargo home.

```sh
python3 ports/acceptance/gate.py build "$config" "$new_run"
```

Every output must be new. A failed build is not resumed/accepted in place.
The build preserves the entropy example's explicit getrandom custom backend;
it does not silently enable one for other applications. The same owned libc
source digest is required across every component. Both profiles are accepted
by the builder, but runtime evidence must name the profile actually executed.
The unwind configuration includes seven existing LSP/unwind/cancellation cases.

Use the guarded VM pool workflow: inspect ownership, reserve the older v11
profile, stage `$new_run/payload` while stopped, then boot. Inspect a fresh
screenshot before each keyboard action. Extract the immutable ZIP to **RAM:wl**,
redirect UnZip output, and verify its return code/log. This path is required
by the existing WAT acceptance scripts.

Only after extraction and Shell readiness, start host fixtures:

```sh
python3 ports/acceptance/gate.py fixtures "$new_run"
```

Inspect `$new_run/fixtures/{socket,async,tls}.log` for ready records before
launching `Execute RAM:wl/run-all` in the guest. Fixtures bind loopback only:
18761/18762 async (88 connections),18763/18764 socket/refusal (83
connections, including exactly two deliberate empty closes), 18771–18776 TLS. All have a 300s
accept window; individual socket I/O remains bounded. Ordinary TLS I/O is 5s;
the deliberately stalled cancellation peer waits up to 60s for ClientHello/EOF. Do not start them during
boot/setup or use already occupied ports. After the guest script returns,
signal **only this fixture-controller process** with SIGINT. It joins/reaps
its children and records their return codes; TLS monitoring must continue
through the forced-entropy binary. An expired/failed fixture is not accepted.
The fixture controller's deadline is not a guest process-kill mechanism.

Stop the owned guest after writes complete, collect to a fresh directory, then
release. Check the entire run:

```sh
python3 ports/acceptance/gate.py check "$new_run" "$collected" --machine v11-2
```

The checker verifies retained component inputs and payload hashes, every required
collected file against the pool SHA manifest, matching root start/end receipts,
all child return/status markers, finite socket/async fixture counts, certificate/
entropy/cancellation TLS results, mmap identity and existing strict WAT lifecycle
checks. A fresh start receipt cannot complete an old run. Guest scripts refuse
all pre-existing required report names before writing. Re-running a completed
suite is rejected. A validation file is written only on success and never
replaced. Failure logs remain available; a failed attempt needs a new build/run.

Receipts identify the staged manifest; they are not an independent guest binary
rehash or an attestation against malicious rewriting. Shared SDK/toolchain inputs
are identified by child recipes but are not fully vendored. This is neither a
hermetic build claim nor acceptance of general POSIX mmap, arbitrary executors,
Tokio networking, full smol/Zed, v1 or arm64/Macaros.
