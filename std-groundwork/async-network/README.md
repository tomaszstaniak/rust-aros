# Socket namespace acceptance (x86_64 ABIv11)

This gate precedes a real async reactor. It uses `std::net::TcpStream`, with
explicit nonblocking I/O and the scoped libc socket readiness adapter. It does
not claim async-net, smol, TLS, native Cargo or Zed runtime acceptance.

Build using the pinned private Rust toolchain, the selected `aros-env.toml`
and `x86_64-aros.json`, `-Zbuild-std=std,panic_unwind`, `-Zjson-target-spec`,
`-Cpanic=unwind -Cforce-unwind-tables=yes`, and a crates.io libc path override
pointing at the same build-owned libc-aros source used by std. Never mix a
new application libc with an old prebuilt std and infer its descriptor ABI.
The current verified private alias is `aros-net-20261016`; its resolved sysroot
must be checked before building. Generated sysroots/artifacts belong outside Git.

Run `python3 fixture.py` on the host immediately before the guest test. It
listens on 127.0.0.1:18761, accepts exactly 83 connections, imposes a 45-second
accept deadline and three-second per-connection timeout, and exits after joining
all fixture workers. In the QEMU user-networked guest, run the binary with
numeric endpoint `10.0.2.2:18761`, stack 1048576, and stdout redirected to a new
RESULTS: filename. Require both the final guest PASS marker and host JSON
`completed=83, errors=[]`. A missing final marker is failure, even if an AROS
Shell script continued after a Rust error. No DNS setup or system settings change.

The native gate covers pending WouldBlock, connect timeout, writable/readable
readiness, deadline, vectored output, delayed response, EOF, invalid descriptors,
80 repeated lifecycles, stdin preservation, ordinary DOS file read/write/dup/drop,
no-argument FIOCLEX, unsupported poll events, foreign-task DNS/socket refusal and
queued foreign drop followed by an owner operation. Temporary files use RAM:;
this named guest has no Unix /tmp. File::try_clone is explicitly expected to
return EINVAL on this profile: F_DUPFD_CLOEXEC is declared but unimplemented in
its fcntl. Ordinary libc::dup is tested independently. This limitation remains
open; it is not masked as a passing File::try_clone feature.

## Descriptor and ownership contract

- libc-aros socket handles have a private high tag. `AsRawFd` exposes this
  process-local handle, not a bsdsocket.library descriptor. Only libc-aros
  dispatchers accept it. Do not hand it to an unadapted C library or insert it
  into an fd_set. TLS callbacks must use the Rust/libc-aros send/recv boundary.
- read/write/close/fcntl/ioctl and vectored operations route tagged handles to
  BSD socket operations; ordinary DOS handles keep using the existing C SDK.
- Socket operations and library-backed name resolution run only on the task
  that opened SocketBase. Foreign I/O fails ENOTSUP. A foreign close queues
  cleanup until that task's next guarded socket I/O/creation/readiness operation or library shutdown.
  Failed deferred closes retain ownership and make the next operation fail;
  a later call retries. There is no idle-owner wakeup mechanism yet.
- Socket duplication and cross-namespace dup2 return ENOTSUP. General threaded
  std networking, socket inheritance across exec, and arbitrary RawFd C FFI
  are not accepted by this gate.
- The poll adapter supports nonzero POLLIN/POLLOUT requests on tagged sockets.
  Unsupported event requests return ENOTSUP; invalid or DOS descriptors produce
  POLLNVAL. Negative entries are ignored. EOF is readable. It does not provide
  generic file/pipe polling, edge triggering, urgent-data/POLLPRI handling or
  general POLLERR/POLLHUP classification. select exception readiness is not
  misreported as POLLERR.

Evidence and named profile: `docs/evidence/2026-10-16-socket-boundary/`.
