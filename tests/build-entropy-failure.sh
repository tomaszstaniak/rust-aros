#!/bin/sh
# Build the real source adapter with a forced startup failure for native tests.
set -eu
[ "$#" -eq 2 ] || { echo "usage: $0 LIBAROSCOMPAT_ARCHIVE OUTPUT" >&2; exit 1; }
: "${AROS_GCC:?set AROS_GCC to the v11 compiler}"
: "${AROS_SDK:?set AROS_SDK to its matching SDK}"
ROOT=$(cd "$(dirname "$0")/.." && pwd)
CSRC="$ROOT/std-groundwork/libc-aros/csrc"
exec "$AROS_GCC" "--sysroot=$AROS_SDK" -O2 -std=gnu99 \
    -DAROS_JENT_FORCE_FAIL "-I$CSRC/vendor/jitterentropy" \
    "-I$CSRC/vendor/arospkg-entropy" \
    "$ROOT/tests/native/entropy_failure.c" "$CSRC/entropy.c" \
    "$CSRC/jent_adapter.c" "$1" -o "$2"
