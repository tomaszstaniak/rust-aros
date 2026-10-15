#!/bin/sh
# Build an explicitly named, isolated rustup toolchain whose std knows AROS.
#
# rustc finds std's sources under its own sysroot, so this creates a sysroot
# directory that borrows everything from nightly except lib/rustlib/src/rust,
# which is a copy with the AROS patches applied and the AROS libc dropped in.
#
#   ./make-std-toolchain.sh /new/sysroot aros-local-name
#
# Afterwards: cargo +aros-local-name build --release (with build-std = ["std"])
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
if [ "$#" -ne 2 ]; then
    echo "usage: $0 NEW_SYSROOT LOCAL_TOOLCHAIN_NAME" >&2
    exit 1
fi
NEW=$1
NAME=$2
case "$NAME" in
    aros-nightly|stable*|beta*|nightly*|''|*[!a-zA-Z0-9_-]*)
        echo "choose a new local toolchain name (not aros-nightly or an upstream channel)" >&2; exit 1 ;;
esac
[ ! -e "$NEW" ] && [ ! -L "$NEW" ] || { echo "destination already exists: $NEW" >&2; exit 1; }
LIST=$(rustup toolchain list)
if printf '%s\n' "$LIST" | awk '{print $1}' | grep -Fx "$NAME" >/dev/null; then
    echo "toolchain already registered: $NAME" >&2; exit 1
fi
# An explicitly selected installed base is useful offline, but must be the
# exact compiler the std patch was tested against. Never update nightly here.
BASE=${AROS_RUST_BASE:-nightly-2026-08-31}
VERSION=$(rustup run "$BASE" rustc -V) || {
    echo "install the pinned base: rustup toolchain install nightly-2026-08-31 --profile minimal --component rust-src" >&2; exit 1;
}
[ "$VERSION" = 'rustc 1.100.0-nightly (0dfb098f3 2026-08-31)' ] || {
    echo "unsupported base compiler: $VERSION" >&2; exit 1;
}
SYS=$(rustup run "$BASE" rustc --print sysroot)
[ -d "$SYS/lib/rustlib/src/rust/library" ] || { echo "rust-src missing for $BASE" >&2; exit 1; }

# mkdir must fail if another installer wins the destination. Only after it
# succeeds do we own the path and have permission to clean it on failure.
mkdir -p "$(dirname "$NEW")"
mkdir "$NEW"
NEW=$(cd "$NEW" && pwd)
cleanup() { rm -rf -- "$NEW"; }
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

mkdir -p "$NEW/bin" "$NEW/lib/rustlib/src"
# Paths are data, including literal dollar signs, quotes and backticks. Encode
# shell literals rather than interpolating them into executable double quotes.
python3 - "$SYS" "$NEW" <<'PY'
from pathlib import Path
from shlex import quote
import sys
base, dest = map(Path, sys.argv[1:])
for binary in (base / 'bin').iterdir():
    args = [str(binary)]
    if binary.name == 'rustc':
        args += ['--sysroot', str(dest)]
    wrapper = dest / 'bin' / binary.name
    wrapper.write_text('#!/bin/sh\nexec ' + ' '.join(map(quote, args)) + ' "$@"\n')
    wrapper.chmod(0o755)
PY
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

# Register only a complete, usable sysroot. A failed patch never creates a
# rustup entry and never replaces an existing sysroot.
"$NEW/bin/rustc" -V
rustup toolchain link "$NAME" "$NEW"
trap - EXIT HUP INT TERM
echo
echo "toolchain '$NAME' ready: $VERSION"
echo "sysroot: $NEW"
