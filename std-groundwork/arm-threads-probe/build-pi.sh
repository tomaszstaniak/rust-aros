#!/bin/sh
# Build the probe with the exact target spec, linker wrapper, Jonx bridge and pthread objects
# that VectorCraft's vc.py generated for aros-aarch64-abiv1-raspi.
#   build-pi.sh <toolchain> <vectorcraft build dir> <target dir>
set -eu
tc=$1; vcb=$2; td=$3
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS CARGO_BUILD_RUSTFLAGS
cd "$(dirname "$0")"
CARGO_TARGET_DIR="$td" cargo +"$tc" build --release --target "$vcb/aarch64-v1-raspi.json" \
  -Zbuild-std=std,panic_abort -Zjson-target-spec
