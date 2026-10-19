#!/usr/bin/env python3
"""Stage libc-aros together with the unchanged upstream libc for the host.

`[patch.crates-io] libc` applies to every crate in a build, host build
scripts and proc macros included. A workspace whose host tools use libc
(threadpool's num_cpus, tempfile's getrandom, ...) then compiles those for
macOS or Linux against the AROS bindings and fails. The staged crate
dispatches on target_os: AROS gets libc-aros and its C glue, every other
target the upstream crate of the same version, unchanged.

Usage: libc-dual.py LIBC_AROS_DIR DEST [--cargo-home DIR]
DEST must not exist. The upstream archive is read from Cargo's registry
cache and must match the pinned checksum. Adapted from the PdfCraft port's
dual_libc.py (aros-artcraft), which first needed it.
"""
import argparse
import hashlib
import os
import re
import shutil
import sys
import tarfile
import tempfile
from pathlib import Path

VERSION = "0.2.189"
CHECKSUM = "3eaf3ede3fee6db1a4c2ee091bf8a8b4dccdc6d17f656fb07896ee72867612f2"


class Failure(Exception):
    pass


def upstream_archive(cargo_home):
    archives = list(cargo_home.glob(f"registry/cache/*/libc-{VERSION}.crate"))
    if len(archives) != 1:
        raise Failure(f"need exactly one cached libc-{VERSION}.crate under {cargo_home}; run cargo fetch")
    if hashlib.sha256(archives[0].read_bytes()).hexdigest() != CHECKSUM:
        raise Failure(f"{archives[0]} does not match the pinned checksum")
    return archives[0]


def check_version(aros):
    manifest = (aros / "Cargo.toml").read_text()
    if f'version = "{VERSION}"' not in manifest:
        raise Failure(f"libc-aros is not version {VERSION}; review the host merge before changing its base")


def combine(aros, archive):
    """Turn the copy of libc-aros at `aros` into the dual crate, in place."""
    with tempfile.TemporaryDirectory(prefix="libc-host-") as tmp:
        with tarfile.open(archive) as tar:
            tar.extractall(tmp, filter="data")
        host = Path(tmp) / f"libc-{VERSION}"
        # The AROS bindings become a module; crate attributes stay at the root.
        lines = (aros / "src/lib.rs").read_text().splitlines(keepends=True)
        lines = [line for line in lines if not line.startswith(("#![no_std]", "#![cfg_attr"))]
        # Its submodules stay beside it, not under src/aros/.
        lines = [re.sub(r"^((?:pub )?mod (\w+);)", r'#[path = "\2.rs"]\n\1', line) for line in lines]
        (aros / "src/aros.rs").write_text("".join(lines))
        root = (host / "src/lib.rs").read_text()
        marker = "    if #[cfg(windows)] {"
        if root.count(marker) != 1 or root.count("mod new;") != 1 or root.count("pub use new::*;") != 1:
            raise Failure("review the upstream libc platform dispatch before changing its base")
        root = root.replace("mod new;", '#[cfg(not(target_os = "aros"))]\nmod new;')
        root = root.replace("pub use new::*;", '#[cfg(not(target_os = "aros"))]\npub use new::*;')
        root = root.replace(marker, '    if #[cfg(target_os = "aros")] {\n        mod aros;\n'
                                    '        pub use aros::*;\n    } else if #[cfg(windows)] {')
        for path in (host / "src").iterdir():
            if path.name == "lib.rs":
                continue
            dest = aros / "src" / path.name
            if dest.exists():
                raise Failure(f"upstream {path.name} would replace an AROS source file")
            (shutil.copytree if path.is_dir() else shutil.copy2)(path, dest)
        (aros / "src/lib.rs").write_text(root)
        shutil.copy2(host / "Cargo.toml", aros / "Cargo.toml")
        # The AROS build script compiles the C glue; the host keeps libc's own.
        shutil.copy2(aros / "build.rs", aros / "aros_build.rs")
        build = (host / "build.rs").read_text()
        marker = "fn main() {"
        if build.count(marker) != 1:
            raise Failure("review the upstream libc build script before changing its base")
        build = build.replace(marker, 'mod aros_build;\n\nfn main() {\n'
                              '    println!("cargo:rustc-check-cfg=cfg(target_os, values(\\"aros\\"))");\n'
                              '    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("aros") {\n'
                              '        aros_build::compile();\n        return;\n    }', 1)
        (aros / "build.rs").write_text(build)
        p = aros / "aros_build.rs"
        text = p.read_text()
        if text.count("fn main() {") != 1:
            raise Failure("libc-aros build.rs changed shape; review aros_build.rs")
        p.write_text(text.replace("fn main() {", "pub fn compile() {", 1))
        for name in ("LICENSE-APACHE", "LICENSE-MIT", "README.md"):
            if (host / name).exists():
                shutil.copy2(host / name, aros / ("host-" + name))


def stage(source, dest, cargo_home):
    if dest.exists() or dest.is_symlink():
        raise Failure(f"{dest} exists; choose a new destination")
    check_version(source)
    archive = upstream_archive(cargo_home)
    dest.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="libc-dual-", dir=dest.parent) as tmp:
        staged = Path(tmp) / "libc"
        shutil.copytree(source, staged, ignore=shutil.ignore_patterns("target", ".DS_Store"))
        combine(staged, archive)
        staged.rename(dest)


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("libc_aros", type=Path)
    p.add_argument("dest", type=Path)
    p.add_argument("--cargo-home", type=Path, default=Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")))
    args = p.parse_args()
    try:
        stage(args.libc_aros.resolve(), args.dest.resolve(), args.cargo_home)
    except Failure as error:
        sys.exit(f"libc-dual: {error}")


if __name__ == "__main__":
    main()
