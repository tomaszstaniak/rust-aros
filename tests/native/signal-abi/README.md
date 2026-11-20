# Mainline AROS signal ABI probe

Compile sdk.c with the ABIv1 compiler and SDK selected for the Rust target.
Then cross-build this manifest with the corresponding AROS Rust/SDK configuration
and run the executable. It must print SIGNAL_ABI_OK and return successfully after
restoring the original SIGUSR2 action. The callback accepts a null information
pointer, as mainline raise can supply one.
