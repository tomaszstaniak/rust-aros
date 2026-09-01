# Why the target definition looks like this

`x86_64-aros.json` starts from `x86_64-unknown-linux-gnu`, because AROS uses
the same SysV ABI on x86_64. Everything below is a change that a build failure
forced, recorded so the next person does not have to rediscover it.

`x86_64-aros.json` starts from `x86_64-unknown-linux-gnu` (same SysV ABI) with
five AROS-specific changes; each one was a build failure first:

- `"relocation-model": "static"` and PIE off. AROS executables are `ET_REL`
  and the loader relocates them itself; PIC is wrong here and breaks the C
  side too.
- `"has-thread-local": false` — no ELF TLS in an `ET_REL` binary.
- `"panic-strategy": "abort"` — there is no unwinder to build against yet.
- `"no-default-libraries": false` — we *want* the SDK's crt and libc: they
  provide `__startup_entry` (which calls our `main`) and the C functions.
- `"post-link-args": ["-Wl,--no-gc-sections"]` — rustc adds `--gc-sections`
  for the gnu-cc flavour, and AROS' relocatable link has no entry point to
  anchor it, so ld refuses: *"gc-sections requires either an entry or an
  undefined symbol"*.

Also note `"crt-static-default"` requires `"crt-static-respected"`, and a
`.json` target now needs `-Z json-target-spec` as well as `-Z build-std`.

The program exposes `#[no_mangle] extern "C" fn main`, so the SDK's C startup
stays in charge — the same entry path our C programs use. Do not strip the
result (see `docs/aros-reference.md` in the AROS workspace).

### `--allow-multiple-definition`

`libcrt.a` and `libexec.a` both define `close` (and `open`). Any program that
mixes C file I/O with exec.library calls therefore fails to link. The C library
is linked first, so allowing the duplicate keeps the POSIX version, which is
the one you want; this was checked at runtime, not just at link time.

## Verified behaviour of the toolchain

Measured on AROS One 1.3 while bringing this up: the ELF loader copes with an
8 MB `.text`, a 70 MB `.bss`, 24 000 sections and 165 000 relocations. What it
does not cope with is a fully stripped file - `LoadSeg` succeeds but the
`.text` relocations are silently skipped, and the program dies in `strlen`
inside its first `OpenLibrary` call, before `main`. This applies to C and Rust
binaries alike.
