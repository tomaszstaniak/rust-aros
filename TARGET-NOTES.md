# Why the target definition looks like this

## One target spec, one SDK profile per AROS build

The Rust side — this target spec, the codegen, the runtime crate — is shared
across AROS builds. What is *not* shared is the ABI of calls into AROS's own
libraries, and mixing two SDKs produces a crash that looks like anything but
what it is.

### The register-call ABI differs between AROS builds

`proto/exec.h` defines `AllocMem` as an inline function, and how that inline
reaches exec.library depends on the AROS build the headers came from:

| | how `SysBase` is passed |
|---|---|
| AROS One 1.3 | in `%r12`, register-call: `mov %rax,%r12; call *-0x108(%rax)` |
| mainline | as an ordinary third argument: `mov %rax,%rdx; jmp *-0x108(%rdx)` |

So `csrc/execglue.c` must be compiled with the **same** SDK the program is
linked against. Compile it against one and link against the other and the
program dies inside `AllocMem` — with a PC in a Kickstart module, which reads
exactly like a broken startup or an uninitialised `SysBase`, and is neither.

That is not hypothetical: it happened here on 2026-09-04. `aros/build.rs` tried
to read the toolchain out of the target spec, assuming `TARGET` holds a path
ending in `.json`. Cargo passes the target *name*, so the branch never fired,
`AROS_GCC`/`AROS_SDK` from the environment silently decided the compiler, and
a mainline-linked binary carried AROS One glue. `build.rs` now requires those
two variables and says so if they are missing.

**`AROS_GCC`, `AROS_SDK` and the `linker` in your target spec must all name the
same AROS build.** Nothing checks this for you.

### Mainline's SDK is laid out differently

Each of these was a link error first:

- **The C library is split.** AROS One has one `libcrt.a`. Mainline has
  `libstdc.a` (`free`, `exit`), `libstdcio.a` (which also supplies
  `__aros_getbase_StdCIOBase`) and `libposixc.a` (`posix_memalign`). Linking
  needs `-lposixc -lstdcio -lstdc -lautoinit`.
- The naming matches what the aarch64 Raspberry Pi distribution ships, so this
  is mainline's layout rather than a local quirk.
- **SDL2 needs building before you can link against it.** It is in mainline's
  contrib (`contrib/SDL2`, with `SDL2_image`, `SDL2_mixer` and `SDL2_ttf`), but
  a plain system build leaves only the mmakefiles — no `libSDL2.a` reaches the
  SDK. So `plasma` cannot be linked against a fresh mainline SDK until contrib
  SDL2 is built; the runs recorded here used `template`, which needs no SDL.

### Mainline's TLS is twenty-eight years newer

Not a target-spec matter, but it decides what is worth attempting from Rust:

| | AROS One 1.3 | mainline (2026-09-04) |
|---|---|---|
| OpenSSL | 1.1.0h, 27 Mar 2018 | **4.0.1, 9 Jun 2026** |
| mbedTLS | — | **3.6.7** |

A TLS 1.3 handshake to a public host, verified against
`ENV:SYS/Certificates/ca-bundle.crt`, succeeds from inside mainline. On AROS
One, anything speaking to a modern server is starting from an OpenSSL that
predates TLS 1.3 support in the wild.

### What has been run where

- **AROS One 1.3**: everything in this repository, including the experimental
  `std`, `plasma` and the SDL2 path.
- **Mainline**: `template` builds and runs completely — `println!` through
  dos.library, `Vec<String>` on the heap, and `exec::Mem` through
  exec.library — once the glue is compiled with mainline's own SDK.
- A binary built for one and copied to the other does **not** run. An AROS One
  `plasma` dies on mainline in `Exec_49_FindTask`, before reaching our code.

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
