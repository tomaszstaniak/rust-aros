# Mio 1.2.1 on AROS ABIv1

Prepare the pinned PomAROS crate store from the base in the patch header.
Apply this series in the mio-1.2.1 directory and select it in Cargo
`[patch.crates-io]`. Update the Cargo lock to 1.2.1 and use this
repository's ABI-aware dual libc. Both overrides are needed.

For a native smoke test, build probe.rs as a binary with Mio features
net, os-poll and os-ext on x86_64 ABIv1. On a guest with networking up
the executable must print MIO_AROS_OK. The second poll waits with a
registered socket whose mask Mio cleared after delivery. The old libc
returned Unsupported; with the fix the wait completes without a duplicate
event. The probe also checks a registered Waker. This is a local
loopback test, with no remote server or external Internet needed.

This follow-up depends on our descriptor API. Revisit the patch when
both toolchains share that interface.
