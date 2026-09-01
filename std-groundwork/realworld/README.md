# Real-world crates on the AROS `std`

Two programs nobody wrote for AROS, built with the `aros-nightly` toolchain
and run on AROS One 1.3.

## ripgrep 14.1.1 — works

```
9.AROS:System> AMIDEV:rg --version
ripgrep 14.1.1 (rev 4649aa9700)
simd(runtime):+SSE2,-SSSE3,-AVX2

9.AROS:System> AMIDEV:rg -n --no-heading Assign AROS:S
AROS:S/user-startup:6:Assign TBImages: SYS:Classes/ToolbarImages/Default >NIL:
AROS:S/Startup-Sequence:16:Assign "T:"        "RAM:T"
...
```

Recursive directory walk, worker threads, regex, line numbers, argument
parsing — all of it. Two things were needed, and both are the point of the
exercise:

* **`[patch.crates-io] libc = ...` in the application.** Upstream `libc` has no
  `target_os = "aros"`, so any crate depending on it directly fails with
  hundreds of errors. Patching our libc in fixes it. This is the strongest
  argument for eventually upstreaming AROS into the `libc` crate.
* **`mmap`.** ripgrep memory-maps files to search them. `csrc/compat.c` now
  emulates it: anonymous maps are allocations, read-only file maps are reads.
  A shared writable file map is **refused** (`ENOTSUP`) rather than silently
  dropping the writes, and `mprotect` refuses anything it cannot enforce.

Note the size: 66 MB as built (ripgrep's release profile keeps debug info),
10 MB after `--strip-unneeded --remove-section .comment`.

## tokio 1.x — runtime yes, async networking no

```
tokio on AROS
  channel: [0, 1, 2]
  timer slept 202 ms
done
```

The multi-threaded runtime, task spawning, `sync::mpsc` and `time::sleep` all
work. Async networking does not: `mio` has no selector for AROS (it wants
epoll/kqueue) and `socket2` wants constants we do not define. Enabling the
`net` feature fails to compile — it does not fail at runtime, which is the
right way round.

Writing a `mio` backend over `WaitSelect` is the obvious next project for
anyone who wants async networking here.

### The bug this found

`tokio::time::sleep` hung forever at first, and the cause was ours: the compat
layer had `pthread_condattr_setclock` return success without doing anything.
std asserts on that result and then computes condvar deadlines against the
clock it believes it selected, so every timed wait got a monotonic deadline on
a realtime clock — roughly 57 years in the past. Nothing failed; it simply
never woke up.

The fix was in two parts: the stub now returns `ENOTSUP` honestly, and
`patch-std.py` adds AROS to the list of platforms std already knows cannot set
the condvar clock, which makes it use `CLOCK_REALTIME` deadlines.

It is worth stating plainly, because it is the failure mode this whole port
keeps producing: **a stub that lies about success is worse than one that
fails.** The same shape as `open` from exec.library appearing to work.
