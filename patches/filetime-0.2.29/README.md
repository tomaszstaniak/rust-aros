# Filetime 0.2.29 on AROS ABIv1

Verify the crate archive against the SHA-256 in the patch header and apply
the series. Select this crate and the dual libc via Cargo overrides.
Build probe.rs with dependencies on filetime and libc-aros. On an x86_64
ABIv1 guest it checks access to a regular RAM file, a missing path, mtime
readback, and refusal of the no-follow operation. It must print
FILETIME_ACCESS_AROS_OK. The fixture does not create an actual symlink.
