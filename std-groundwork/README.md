# Rust `std` on AROS x86_64

**`std` works.** See `std-patch/README.md` for how to set up the toolchain;
the rest of this file describes the libc groundwork it stands on.

# Towards Rust `std` on AROS x86_64 (groundwork)

Groundwork for a real `std`: a `libc` crate for AROS, generated from the SDK
rather than adapted from another platform, and validated by running against
the actual system.

## Status

`libc-aros` binds **75 functions**, all verified to exist in the SDK link
libraries, plus 106 constants and the type layouts std's platform code needs.
`libc-test` exercises them on AROS One 1.3 and everything it covers passes:

```
[ok] clock_gettime REALTIME     tv_sec=1788295198
[ok] nanosleep 200ms            monotonic delta = 201396000 ns
[ok] open O_WRONLY|O_CREAT      [ok] write
[ok] stat st_size               st_size=24 (wrote 24), st_mode=0o100700
[ok] S_IFMT says regular file   [ok] read back matches
[ok] opendir RAM:               "Clipboards", "T", "ENV" (d_type=4)
[ok] pthread_create             [ok] thread ran and wrote
[ok] mutex lifecycle
```

## Why the bindings were measured, not copied

Every one of these would have compiled cleanly from a Linux template and then
misbehaved at runtime:

| | AROS | Linux |
|---|---|---|
| `O_RDONLY` | **1** | 0 |
| `EAGAIN` | **35** (BSD-style) | 11 |
| `time_t` | **4 bytes** — times end in 2038 | 8 |
| `pid_t` | **8 bytes** | 4 |
| `pthread_t` | **4 bytes** | 8 (pointer) |
| `mode_t` | 2 bytes | 4 |
| `CLOCK_REALTIME` | **2** | 0 |
| `struct timespec` | `tv_sec` is 4 bytes, `tv_nsec` at offset 8 | both 8 |

Struct layouts came from DWARF emitted by the AROS cross compiler; constants
were expanded from the SDK headers and evaluated. The tooling is in `probe/`.

## What this found out about AROS

* **There is no POSIX `open` symbol.** The plain `open` in the SDK belongs to
  exec.library and is a different function; the C library exports its own as
  `__open_CrtBase_wrapper`. Binding the obvious name gives an `open` that
  "succeeds" and then every write and stat fails. Our binding uses
  `#[link_name]`.
* **`libcrt.a` and `libexec.a` both define `close`** (and `open`), so any
  program mixing C file I/O with exec calls needs
  `-Wl,--allow-multiple-definition`; the C library is linked first, which is
  the version you want.
* **pthreads live in `libpthread.a`** and are complete: creation, join, TLS
  keys, mutexes, condition variables with timeouts.
* **Absent from the SDK:** `O_CLOEXEC`, `O_DIRECTORY`, `O_NOFOLLOW`,
  `AT_FDCWD`, `_SC_PAGESIZE`, `_SC_NPROCESSORS_ONLN`.

## What remains for `std`

Measured against the calls std's platform layer makes:

* **Reusable almost as-is:** files, io, time, environment, threads, sync,
  alloc, thread-local storage. AROS has real pthreads and real POSIX I/O.
* **Needs stubbing first:** the stack guard machinery, because `mmap`,
  `mprotect` and `getpagesize` do not exist.
* **Needs an AROS-native implementation** — and both now have a working
  prototype below: `process`, since there is no `fork`, and `net`, since
  sockets are not libc symbols — they are calls through `bsdsocket.library`'s
  base. **That part is now solved in principle:** `net-test/` opens a TCP
  connection from Rust and completes an HTTP exchange with the host.

## Processes: `process-test/`

There is no `fork`, but `std::process::Command` does not need one: it needs
"run this, capture its output, give me the exit code". dos.library's
`SystemTagList` does exactly that, and every call involved is a plain symbol,
so no C glue is required. Verified on AROS One 1.3:

```
[ok] echo hello            status=0 stdout=27 bytes  | hello from a child process
[ok] version               status=0 stdout=32 bytes  | Kickstart 51.51, Workbench 40.0
[ok] list RAM:             status=0 stdout=17 bytes  | Clipboards / T / ENV
sync status: echo=0  'quit 10'=10
```

Recipe: open a uniquely named `PIPE:` file for writing, pass it as
`SYS_Output` with `SYS_Asynch`, then open the same name for reading — the
child's exit closes the writer, so the reader sees EOF. For the exit code, run
synchronously (no `SYS_Asynch`): the return value is the child's return code.
Two caveats for a real port: a command that does not exist yields status 0
from the shell (check the path before spawning), and stdin must be given
explicitly (`NIL:`) or the child inherits the parent's console.

## Sockets: `net-test/`

bsdsocket.library is called through its library base, kept in a register the
C compiler manages (`r12` on x86_64, LVO offset `-40` for `socket`). Instead
of teaching Rust that convention, `csrc/sockglue.c` wraps each call in an
ordinary SysV function, `build.rs` compiles it with the AROS cross compiler
taken from the target spec, and `libnet.a` opens `SocketBase` at startup.
Verified against a web server on the host:

```
[ok] SocketBase opened by libnet at startup
[ok] socket() = 0
[ok] connect(10.0.2.2:8080) = 0 errno 0
[ok] send() = 33
[ok] recv() = 200 bytes, first line: HTTP/1.0 200 OK
```

Two details for a `std::net` port: `sockaddr_in` is the BSD layout with a
leading `sin_len` byte, and the socket errno comes from `Errno()`, not the C
library's `errno`.

A first milestone of "std minus process and net" therefore looks reachable:
patch the `std` sources from `rust-src` to treat AROS as a unix family target,
point them at this `libc`, and build with `-Z build-std=std`. That probably
needs no fork of the compiler itself.

## Layout

| Path | What it is |
|---|---|
| `libc-aros/` | The `libc` crate for AROS. |
| `libc-test/` | Runtime validation; `cargo +nightly run --release`. |
| `probe/` | Layout and constant extraction (DWARF + preprocessor). |
| `net-test/` | TCP over bsdsocket.library from Rust, with the C glue it needs. |
| `process-test/` | Child processes with captured output and exit codes via SystemTagList. |

Requires the target definition and runtime crate from `../rust-aros`.
