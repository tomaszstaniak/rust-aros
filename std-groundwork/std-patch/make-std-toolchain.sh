#!/bin/sh
# Build a rustup toolchain named "aros-nightly" whose std knows about AROS.
#
# rustc finds std's sources under its own sysroot, so this creates a sysroot
# directory that borrows everything from nightly except lib/rustlib/src/rust,
# which is a copy with the AROS patches applied and the AROS libc dropped in.
#
#   ./make-std-toolchain.sh [dir]     default: ../../../aros-toolchain-sysroot
#
# Afterwards: cargo +aros-nightly build --release   (with build-std = ["std"])
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
NEW=${1:-$HERE/../../../aros-toolchain-sysroot}
NEW=$(mkdir -p "$NEW" && cd "$NEW" && pwd)
SYS=$(rustup run nightly rustc --print sysroot) || { echo "install nightly first: rustup toolchain install nightly --component rust-src" >&2; exit 1; }
[ -d "$SYS/lib/rustlib/src/rust/library" ] || { echo "rust-src missing: rustup component add rust-src --toolchain nightly" >&2; exit 1; }

rm -rf "$NEW/bin" "$NEW/lib"
mkdir -p "$NEW/bin" "$NEW/lib/rustlib/src"
for b in "$SYS"/bin/*; do
  n=$(basename "$b")
  printf '#!/bin/sh\nexec "%s" "$@"\n' "$b" > "$NEW/bin/$n"; chmod +x "$NEW/bin/$n"
done
# rustc must report this directory as its sysroot, so cargo -Z build-std reads our sources
printf '#!/bin/sh\nexec "%s/bin/rustc" --sysroot "%s" "$@"\n' "$SYS" "$NEW" > "$NEW/bin/rustc"; chmod +x "$NEW/bin/rustc"
for d in "$SYS"/lib/rustlib/*; do
  case "$(basename "$d")" in src) ;; *) ln -s "$d" "$NEW/lib/rustlib/$(basename "$d")" ;; esac
done
for f in "$SYS"/lib/*.dylib "$SYS"/lib/*.so; do [ -e "$f" ] && ln -s "$f" "$NEW/lib/"; done 2>/dev/null || true

cp -R "$SYS/lib/rustlib/src/rust" "$NEW/lib/rustlib/src/rust"
LIB="$NEW/lib/rustlib/src/rust/library"
rm -rf "$LIB/libc-aros"; mkdir -p "$LIB/libc-aros"
cp -R "$HERE/../libc-aros/src" "$HERE/../libc-aros/csrc" "$HERE/../libc-aros/build.rs" "$HERE/../libc-aros/Cargo.toml" "$LIB/libc-aros/"
grep -q "libc-aros" "$LIB/Cargo.toml" || python3 - "$LIB/Cargo.toml" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.replace("[patch.crates-io]\n", "[patch.crates-io]\n# AROS: libc generated from the AROS SDK (rust-aros/std-groundwork)\nlibc = { path = 'libc-aros' }\n", 1)
open(p, "w").write(s)
PY
python3 "$HERE/patch-std.py" "$LIB/std/src"

rustup toolchain link aros-nightly "$NEW"
echo
echo "toolchain 'aros-nightly' ready: $(rustup run aros-nightly rustc -V)"
echo "sysroot: $NEW"
