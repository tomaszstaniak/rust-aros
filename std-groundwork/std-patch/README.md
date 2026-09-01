# Rust `std` for AROS x86_64

`std` builds and runs on AROS. A plain Rust program — no `no_std`, no
`unsafe`, nothing AROS-specific in the source — passes on AROS One 1.3:

```
[ok] thread::sleep + Instant      [ok] SystemTime is sane
[ok] fs::write / read_to_string   [ok] fs::metadata len
[ok] fs::read_dir RAM:            [ok] fs::remove_file
[ok] 4 threads, Arc<Mutex>        [ok] mpsc channel
[ok] HashMap                      [ok] process::Command output
[ok] net::TcpStream HTTP
```

and `cargo add serde serde_json anyhow` works unmodified: `std-hello` serialises
a struct to JSON, writes it to `RAM:`, reads it back and prints an `anyhow`
error chain.

## Setting it up

```sh
rustup toolchain install nightly --profile minimal --component rust-src
./make-std-toolchain.sh          # creates the "aros-nightly" toolchain
cd ../std-hello
cargo +aros-nightly run --release
```

`make-std-toolchain.sh` builds a sysroot that borrows everything from nightly
except `lib/rustlib/src/rust`, which is a copy with `patch-std.py` applied and
the AROS `libc` crate dropped in. rustc finds std's sources under its own
sysroot, which is why a plain `-Z build-std` cannot be pointed elsewhere and a
linked toolchain is used instead. Nothing in `~/.rustup/toolchains/nightly` is
modified.

A project then needs, in `.cargo/config.toml`:

```toml
[unstable]
build-std = ["std", "panic_abort"]
json-target-spec = true
```

## What the patch does

`patch-std.py` adds `target_os = "aros"` to `std` as a unix-family target.
Each edit sits next to an existing small target (usually haiku) so the diff
stays reviewable:

| Area | Change |
|---|---|
| `os::aros` | `raw` types and `MetadataExt`, from `os::haiku` minus creation time |
| args | stored from `main`'s argc/argv, as on haiku |
| random | `getentropy` (from `libc-aros/csrc/compat.c`, **not cryptographic**) |
| `current_exe` | `argv[0]`; there is no `/proc` |
| thread names | no-op |
| errno | `___geterrnoptr()` |
| fd sanitising | skipped: it polls fds 0–2, and our `poll` only knows sockets |
| null device | `NIL:` |
| `read_output` | thread per pipe instead of `poll` on non-blocking pipes |
| `process_group` | `pid_t` is 64-bit on AROS |
| supported-OS list in `build.rs` | otherwise everything is `restricted_std` |

## What `libc-aros` supplies underneath

Beyond the measured bindings, `csrc/compat.c` provides what std links against
and the SDK lacks: sockets over `bsdsocket.library` (called through the
library base, so C has to do it), `getaddrinfo` over `gethostbyname` (this
bsdsocket has no `getaddrinfo`), the `*at()` family for `AT_FDCWD`, `poll` over
`WaitSelect` (sockets only), `pread`/`pwrite`/`readv`, `strerror_r`, and
ENOSYS stubs for `chroot`, `mkfifo`, `setpgid`, `setgroups`, `socketpair`.

`fork` is `vfork`, bound **directly** to `__vfork_CrtBase_wrapper`. It cannot
go through a C wrapper: vfork returns twice into the same stack frame, and a
wrapper that returns leaves the child running on a frame the parent already
left. That one cost an hour. std only `dup2`s, closes and `chdir`s between
vfork and exec, which vfork permits, and the child/exec/waitpid/pipe sequence
was verified by hand before trusting it.

## Known gaps

* `Command::new("echo")` with a bare name fails: `execvp` does not search the
  AmigaDOS path. Give a full name such as `C:echo`.
* `getentropy` is not a real entropy source. `HashMap` seeding is fine; do not
  use `std`'s randomness for keys.
* `poll` only works on sockets. `std::net` timeouts use it; nothing else does.
* Nightly only, and the patch targets the `rust-src` shipped with
  `rustc 1.100.0-nightly (2026-08-31)`. Other nightlies may need it adjusted.
* `panic = abort` still — no unwinding.
