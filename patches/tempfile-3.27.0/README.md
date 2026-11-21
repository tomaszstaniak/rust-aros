# tempfile 3.27.0 on AROS x86_64 ABIv1

Apply the series after the PomAROS crate-store patch identified in the
patch header. The ABIv1 crate exposes temporary directory APIs for
scoped staging. Other targets retain the existing crate API.

Build probe.rs with an exact tempfile 3.27.0 dependency and the patched
registry override. The probe creates a directory on RAM Disk, writes and
reads a staging file, then checks that closing the directory removes it.
