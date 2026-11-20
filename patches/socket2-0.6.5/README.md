# Socket2 0.6.5 for our AROS ABIv1 libc

Prepare the crate with PomAROS crate store at the revision in the patch
header and apply this series. Use it together with this repository's
dual libc via Cargo overrides. Compile sdk.c against the ABIv1 SDK and
build probe.rs with socket2's all feature. The probe must print
SOCKET2_AROS_OK after comparing a UDP loopback payload and checking
the two unsupported message operations. The ABI declarations do not
validate every socket option at runtime.
