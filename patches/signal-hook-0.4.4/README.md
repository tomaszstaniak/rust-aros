# signal-hook 0.4.4 on x86_64 ABIv1

Apply the series to the crates.io release and pair it with the matching
libc-aros and the signal-hook-registry 1.4.6 AROS series. On mainline
posixc, sigprocmask returns ENOSYS. The patch skips that call while
retaining the existing re-raise and abort fallback.

Build probe.rs as a binary with signal-hook 0.4.4 and libc 0.2.189
under those overrides. It checks same-task SIGUSR2 notification and
unregistration. It also checks that MSG_DONTWAIT returns WouldBlock
on an empty UDP socket and receives the expected loopback payload
once the datagram is queued. The probe does not exercise UnixStream
pairs, SIGCHLD, process supervision or terminal startup.
