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
rustup toolchain install nightly-2026-08-31 --profile minimal --component rust-src
./make-std-toolchain.sh /absolute/new/sysroot aros-local
cd ../std-hello
cargo +aros-local build --release
```

The destination must not exist and the toolchain name must be new. The installer
refuses the shared `aros-nightly` name, checks the exact compiler version before
creating output, and registers the new toolchain only after patching succeeds.
It does not update Rust or replace a previous sysroot. To use an already
installed base offline, set `AROS_RUST_BASE=nightly`; the same exact-version
check still applies. Existing users of `aros-nightly` can keep using it.

`make-std-toolchain.sh` builds a sysroot that borrows everything from the pinned nightly
except `lib/rustlib/src/rust`, which is a copy with `patch-std.py` applied and
the AROS `libc` crate dropped in. rustc finds std's sources under its own
sysroot, which is why a plain `-Z build-std` cannot be pointed elsewhere and a
linked toolchain is used instead. Nothing in `~/.rustup/toolchains/nightly` is
modified. Keep the base installation available: the generated toolchain refers
to its binaries and host libraries. Switching Cargo's `+toolchain` selects a
different sysroot without changing other projects. To remove only this new
registration, use `rustup toolchain uninstall aros-local`; archive or remove the
explicit sysroot directory separately after checking that no consumer uses it.

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
| random | `getentropy` (from `libc-aros/csrc/entropy.c`, over the pinned jitterentropy source) |
| `current_exe` | `argv[0]`; there is no `/proc` |
| thread names | no-op |
| errno | `___geterrnoptr()` |
| fd sanitising | skipped: it polls fds 0–2, and our `poll` only knows sockets |
| null device | `NIL:` |
| `read_output` | thread per pipe instead of `poll` on non-blocking pipes |
| `process_group` | `pid_t` is 64-bit on AROS |
| condvar clock | AROS joins the platforms that cannot `pthread_condattr_setclock`, so deadlines use `CLOCK_REALTIME` |
| supported-OS list in `build.rs` | otherwise everything is `restricted_std` |

## What `libc-aros` supplies underneath

Beyond the measured bindings, `csrc/compat.c` provides what std links against
and the SDK lacks: sockets over `bsdsocket.library` (called through the
library base, so C has to do it), `getaddrinfo` over `gethostbyname` (this
bsdsocket has no `getaddrinfo`), the `*at()` family for `AT_FDCWD`, `poll` over
`WaitSelect` (sockets only), `pread`/`pwrite`/`readv`, `strerror_r`, and
ENOSYS stubs for `chroot`, `mkfifo`, `setpgid`, `setgroups`, `socketpair`.

`bsdsocket.library` is optional. `compat.c` defines `SocketBase` itself and
opens the library at startup without insisting: referencing the SDK's
`SocketBase` pulls `net.lib`'s autoinit, which shows "Unable to open
bsdsocket.library version 4 or later" and exits, so every std program needed
a running TCP/IP stack just to start. Without a stack, socket calls now fail
with `ENETDOWN`. A stack started after the program is not picked up.

`libc-aros` links `crt` before `net` for the same reason: `net.lib` has its
own `strerror`, which asks `bsdsocket.library` through `SocketBase` and
crashes when it is NULL. With `crt` first, formatting an `io::Error` works
without a stack.

`std-net-std` tests this path (`std::net::TcpStream` only, so sockets go
through `compat.c`; `net-test` links its own glue and does not). On ABIv11,
2026-09-24: without a stack it reports `Network is down (os error 50)` and
exits normally; with AROSTCP running it fetches a page over HTTP.

`fork` is `vfork`, bound **directly** to `__vfork_CrtBase_wrapper`. It cannot
go through a C wrapper: vfork returns twice into the same stack frame, and a
wrapper that returns leaves the child running on a frame the parent already
left. That one cost an hour. std only `dup2`s, closes and `chdir`s between
vfork and exec, which vfork permits, and the child/exec/waitpid/pipe sequence
was verified by hand before trusting it.

## Known gaps

* `Command::new("echo")` with a bare name fails: `execvp` does not search the
  AmigaDOS path. Give a full name such as `C:echo`.
* The x86_64/v11 entropy adapter now uses arospkg's jitterentropy port, with
  source failures propagated. Rebuild an isolated sysroot to pick it up: older
  installed sysroots still contain the clock/random implementation. Runtime
  scope and getrandom 0.3's explicit custom backend are described in
  `../entropy-std/README.md`. Other getrandom versions are not implicitly supported.
* `poll` only works on sockets. `std::net` timeouts use it; nothing else does.
* `mmap` is emulated: anonymous maps are allocations, read-only file maps are
  reads, and writable shared file maps are refused. `mprotect` enforces
  nothing and refuses anything but read/write.
* Nightly only, and the patch targets the `rust-src` shipped with
  `rustc 1.100.0-nightly (2026-08-31)`. Other nightlies may need it adjusted.
* `panic = abort` still — no unwinding.
