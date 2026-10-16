# x86_64 v11 async networking follow-up

Updated 2026-10-16. [Socket boundary evidence](../evidence/2026-10-16-socket-boundary/REPORT.md).

Accepted prerequisite: private tagged socket handles, nonblocking std TCP,
bounded socket IN/OUT poll, owner-task guard and deferred foreign drop cleanup.
Native entropy/process/LSP regressions pass on the named older v11-2 profile.
This is not a real async reactor or HTTPS runtime milestone.

Next gates:

1. Adapt exact pinned polling/async-io/async-net inputs. Provide owner-task
   selector notification through Exec signals, preserve oneshot/level semantics,
   test lost-wakeup races, cross-thread future notification and timer deadlines.
   Remove assumptions about a worker thread driving the opening task's SocketBase.
2. Accept the bounded real async-net consumer: concurrent logical tasks, delayed
   host reply, refusal, EOF, cancelled future/drop and repeated lifecycles.
3. Add opaque stepped Mbed TLS3.6.7 + jitterentropy3.7.0; require chain/name/date
   verification, trusted/wrong-name/expired/untrusted cases and entropy failure.
   Static build/link experiments are already recorded separately. Old v11
   OpenSSL1 is not selected; a newer OpenSSL requires its own v11 build.
4. Restore ordinary std File::try_clone on this older profile. Its fcntl declares
   F_DUPFD_CLOEXEC but returns EINVAL; ordinary dup works. Design the duplication,
   flag and concurrent-exec contract before adding a non-atomic fallback.
5. Broader socket duplication, idle-owner close draining, worker-task I/O,
   socket/exec inheritance and arbitrary C RawFd interop require separate design
   and runtime acceptance. Do not silently expose a tagged handle as raw BSD fd.

v1, arm64/Macaros and full Zed/GPUI remain later work. No unsupported-platform
conclusion is implied by this adapter's deliberate current restrictions.
