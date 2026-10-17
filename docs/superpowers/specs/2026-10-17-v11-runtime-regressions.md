# Approved v11 runtime regression scope

User approved three isolated regressions on older x86_64 ABIv11: pthread detach
before/after completion with slot reuse; parking_lot realtime timeout/wakeup;
Rayon global pool shutdown and actual process exit. Compare before/after each
corresponding Photocraft patch on the same unwind profile. No Photocraft rebuild,
shared SDK edit, v1 or arm64 work. Work stays in the original Rust repository.

Build immutable private copies; retain compiler/SDK/source/patch/binary hashes.
Use real dependencies, not mocked runtime behavior. A host receipt after the
Shell regains control proves process exit; a marker before main returns does
not. Every case has a finite host deadline, independent of guest clock rate.
Timeout is a recorded observation, not success. Stop only the owned VM after
bounded observation; never unload live worker code with forced guest _Exit.
