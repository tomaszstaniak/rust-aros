# Towards Rust `std` on AROS x86_64

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
* **Needs an AROS-native implementation:** `process`, since there is no `fork`
  (use dos.library's `CreateNewProc`/`SystemTagList`), and `net`, since
  sockets are not libc symbols at all — `libnet.a` only opens
  `bsdsocket.library`, and the calls go through the library base.

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

Requires the target definition and runtime crate from `../rust-aros`.
