#!/bin/sh
# Generate x86_64-aros.json for this machine.
#
# Finds the AROS cross toolchain and SDK, or takes them from the environment:
#   AROS_GCC   path to x86_64-aros-gcc
#   AROS_SDK   path to the SDK (the directory holding include/ and lib/)
set -e
cd "$(dirname "$0")"

find_gcc() {
    [ -n "$AROS_GCC" ] && { echo "$AROS_GCC"; return; }
    if command -v x86_64-aros-gcc >/dev/null 2>&1; then
        command -v x86_64-aros-gcc; return
    fi
    for d in "$HOME/aros/toolchain" "$HOME/AROS/toolchain" /opt/aros/toolchain \
             /usr/local/aros/toolchain /opt/aros/bin /usr/local/aros/bin; do
        [ -x "$d/x86_64-aros-gcc" ] && { echo "$d/x86_64-aros-gcc"; return; }
    done
}

GCC=$(find_gcc)
if [ -z "$GCC" ]; then
    echo "setup: cannot find x86_64-aros-gcc." >&2
    echo "       Put it on your PATH, or run: AROS_GCC=/path/to/x86_64-aros-gcc ./setup.sh" >&2
    exit 1
fi
case "$GCC" in /*) ;; *) GCC=$(cd "$(dirname "$GCC")" && pwd)/$(basename "$GCC") ;; esac
[ -x "$GCC" ] || { echo "setup: $GCC is not executable" >&2; exit 1; }

SDK="$AROS_SDK"
if [ -z "$SDK" ]; then
    # The SDK usually sits beside the toolchain directory.
    for cand in "$(dirname "$GCC")/../sdk" "$(dirname "$GCC")/../../sdk" \
                "$(dirname "$GCC")/../AROS/Development" ; do
        [ -d "$cand/include" ] && [ -d "$cand/lib" ] && { SDK=$(cd "$cand" && pwd); break; }
    done
fi
if [ -z "$SDK" ] || [ ! -d "$SDK/include" ] || [ ! -d "$SDK/lib" ]; then
    echo "setup: cannot find the AROS SDK (a directory with include/ and lib/)." >&2
    echo "       Run: AROS_SDK=/path/to/sdk ./setup.sh" >&2
    exit 1
fi

sed -e "s|@AROS_GCC@|$GCC|" -e "s|@AROS_SDK@|$SDK|" x86_64-aros.json.in > x86_64-aros.json

# Build scripts that compile C glue (the libc crate) read the toolchain from
# these variables; cargo picks the file up from any project under this tree.
cat > aros-env.toml <<EOF
[env]
AROS_GCC = "$GCC"
AROS_SDK = "$SDK"
EOF

echo "setup: toolchain $GCC"
echo "setup: SDK       $SDK"
echo "setup: wrote     x86_64-aros.json, aros-env.toml"
echo
echo "Next:  cd template && cargo +nightly build --release"
