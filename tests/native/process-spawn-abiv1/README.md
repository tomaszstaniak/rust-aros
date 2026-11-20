# x86_64 ABIv1 process fallback regression

Generate a fresh sysroot with tools/common-sysroot.py and cross-build this
manifest for x86_64 AROS ABIv1 with std and panic_abort. Run the binary on the
matching AROS system. It must print PROCESS_SPAWN_UNSUPPORTED_OK and return to
Shell. This checks spawn of an existing and a missing command and output
capture on the profile whose Unix fork fallback previously aborted.

The fallback returns Unsupported until a native process backend is available.
When that backend is enabled, replace this test with its spawn, wait and error
acceptance cases.
