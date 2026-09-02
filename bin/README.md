# Prebuilt AROS x86_64 binaries

Built with the cross toolchain in this repository, so you can try the examples
without setting one up. Copy them anywhere on your AROS system and run them
from a Shell.

| binary | what it is | needs `std` |
|---|---|---|
| `hello-aros` | the project template: `println!`, `Vec`, `format!` | no |
| `plasma` | SDL2 example, animated 320×200 plasma, any key quits | no |
| `libc-test` | checks the generated `libc` bindings against the running system | no |
| `std-hello` | the experimental `std` path: `std::fs`, `std::thread`, collections | yes |

Run them from a Shell rather than by double-clicking, since all four print to
standard output:

```
1.AROS:> AMIDEV:libc-test
```

## Verified

On AROS One 1.3 (x86_64, ABIv11) under QEMU on macOS/arm64, from these exact
files:

```
libc-test: checking the generated bindings against the real system
  [ok] clock_gettime REALTIME      [ok] opendir RAM:
  [ok] nanosleep 200ms             [ok] readdir names are sane
  [ok] open O_WRONLY|O_CREAT       [ok] pthread_create
  [ok] write                       [ok] thread ran and wrote
  [ok] stat st_size                [ok] mutex lifecycle
  [ok] S_IFMT says regular file
  [ok] read back matches
libc-test: done

plasma: Rust on AROS x86_64 - press a key to quit
plasma: 4528 frames in 15000 ms (301.8 fps at 320x200)
```

Not verified anywhere else. If they run — or fail — on Icaros, on real
hardware, or on a hosted build, that is exactly the report this project needs.

## Paths

These carry no absolute paths from the machine that built them. Rust embeds
`file!()` locations in `.rodata` for panic messages, so a `-Zbuild-std` binary
otherwise ships the full path of the sysroot, of this checkout and of the cargo
registry. That is program data, not debug info — `--strip-debug` and even
`--strip-unneeded` leave every one of them in place. `setup.sh` now writes
`--remap-path-prefix` into `aros-env.toml` as `[build] rustflags`, which turns
them into `/rust`, `/rust-aros` and `/cargo`.

Worth knowing if you copy that: it has to be `[build] rustflags`, not
`[env] RUSTFLAGS`. Cargo reads `RUSTFLAGS` from the ambient environment before
it applies `[env]`, so the latter looks right and silently does nothing.

## A note on stripping

These are shipped unstripped on purpose. A full `x86_64-aros-strip` produces a
binary whose `.text` relocations `LoadSeg` silently skips; it loads and then
dies inside its first `OpenLibrary`, before `main`, which looks precisely like
a bug in the program. `--strip-debug` (or `llvm-strip --strip-debug`) is safe
and is what you want if size or load time matters — a binary carrying full
debug info takes a noticeable while to relocate at load.
