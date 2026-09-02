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

### Why there is no `--allow-multiple-definition`

There was, until the cause was measured. `libexec.a` has exactly three members,
and `exec_regcall_stubs.o` carries **every** exec.library stub in one object --
including `open` and `close`. Referencing any exec function by its own name
therefore drags that whole object in, and its `close` collides with
`crt_close_stub.o` in `libcrt.a`:

```
libexec.a(exec_regcall_stubs.o): in function `close':
  multiple definition of `close'; libcrt.a(crt_close_stub.o): first defined here
```

Link order does not help, because both objects are genuinely required. C never
hits this: `proto/exec.h` makes `AllocMem` a macro over an inline function that
calls through `SysBase`, so no undefined exec symbol is emitted and the archive
member is never pulled. That is why a C test program mixing `open`/`close` with
exec calls links with no special flags at all.

`aros/csrc/execglue.c` gives Rust the same property -- it includes
`proto/exec.h` and re-exports the calls under `aros_glue_*` names, which
`sys.rs` binds with `#[link_name]`. Measured effect on the plasma example:
definitions of `open`/`close` pulled from `libexec.a` went from 2 to 0, and
every crate in this repository now links without the flag.

Keeping the flag off is deliberate. With it, naming an exec function directly
does not fail -- it silently binds `close` to exec.library's, and `open` to
exec's rather than the C library's, which is the documented cause of an `open`
that appears to succeed and is followed by failing writes and stats.

## Verified behaviour of the toolchain

Measured on AROS One 1.3 while bringing this up: the ELF loader copes with an
8 MB `.text`, a 70 MB `.bss`, 24 000 sections and 165 000 relocations. What it
does not cope with is a fully stripped file - `LoadSeg` succeeds but the
`.text` relocations are silently skipped, and the program dies in `strlen`
inside its first `OpenLibrary` call, before `main`. This applies to C and Rust
binaries alike.
