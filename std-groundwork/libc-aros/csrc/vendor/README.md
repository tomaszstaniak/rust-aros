# Entropy source provenance

Imported on 2026-10-15 from tomaszstaniak/arospkg commit
91ac4d8f00bbaf5ac5b057322d3937a61acd1cd7.

- jitterentropy/: upstream jitterentropy-library 3.7.0, copied byte-for-byte
  from arospkg/third_party/jitterentropy. The upstream tarball hash and source
  URL are in jitterentropy/README.arospkg. Use under BSD-3-Clause (LICENSE.bsd).
- arospkg-entropy/: entropy_jent.c and jitterentropy-base-user.h copied unchanged
  from arospkg/src/tls (the header is in jent-aros there), under arospkg's MIT
  licence included alongside them.

jent_adapter.c includes the unchanged adapter under private symbol names and
registers collector cleanup at exit. entropy.c supplies the separate POSIX API.
The jitter core is built with -O0 -std=gnu99 -fwrapv, following arospkg's recipe.
No internal timer thread is enabled. The inherited adapter uses an Exec
semaphore with Forbid/Permit for its lazy initialization; acceptance here is
limited to the tested older x86_64/v11 UP guest. No SMP claim is made.

No timer suitability or health-test failure is overridden. Host tests inject
source failures; native tests exercise the real source and an explicitly
forced initialization failure. This is not an entropy certification or a
complete SP 800-90B assessment. See arospkg's scoped SDK report for prior data.
