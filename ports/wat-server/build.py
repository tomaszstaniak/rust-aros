#!/usr/bin/env python3
"""Build a prepared WAT port; generated lock/config changes stay outside Git."""
import argparse
import os
import json
import tomllib
import hashlib
import tempfile
import shutil
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("workspace", type=Path)
p.add_argument("--toolchain", required=True)
p.add_argument("--aros", action="store_true")
p.add_argument("--cancellation-probe", action="store_true", help="build the deterministic Salsa cancellation probe")
args = p.parse_args()
repo = Path(__file__).resolve().parents[2]
work = args.workspace.resolve()
cargo = ["cargo", "+" + args.toolchain]
config = []
if args.aros:
    config = ["--config", str(repo / "aros-env.toml"), "--config",
              'patch.crates-io.libc.path=' + json.dumps(str(repo / "std-groundwork/libc-aros"))]
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    archives = list((cargo_home / "registry/cache").glob("*/inventory-0.3.24.crate"))
    if len(archives) != 1:
        p.error("inventory 0.3.24 must be fetched into CARGO_HOME first")
    archive = archives[0]
    checksum = "a4f0c30c76f2f4ccee3fe55a2435f691ca00c0e4bd87abe4f4a851b1d4dac39b"
    if hashlib.sha256(archive.read_bytes()).hexdigest() != checksum:
        p.error("inventory archive checksum mismatch")
    dependency = work.parent / (work.name + "-deps") / "inventory-0.3.24"
    patched_sha256 = "2ffb0302f8035019f43d24298d6d949ab08a6476553e31554f55b89e228839b2"
    def tree_hash(root):
        digest = hashlib.sha256()
        for path in sorted(x for x in root.rglob("*") if x.is_file()):
            digest.update(path.relative_to(root).as_posix().encode() + b"\0" + hashlib.sha256(path.read_bytes()).digest())
        return digest.hexdigest()
    if not dependency.exists():
        dependency.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="inventory-stage-", dir=dependency.parent) as temp:
            subprocess.run(["tar", "-xzf", str(archive), "-C", temp], check=True)
            staged = Path(temp) / dependency.name
            subprocess.run(["patch", "-p1", "-i", str(Path(__file__).with_name("inventory-aros.patch").resolve())],
                           cwd=staged, check=True)
            if tree_hash(staged) != patched_sha256:
                p.error("patched inventory tree checksum mismatch")
            staged.rename(dependency)
    if tree_hash(dependency) != patched_sha256:
        p.error("existing inventory tree differs from the reviewed patch; use a new workspace")
    config += ["--config", 'patch.crates-io.inventory.path=' + json.dumps(str(dependency))]
    packages = tomllib.loads((work / "Cargo.lock").read_text())["package"]
    if any(x["name"] == "libc" and x["version"] == "0.2.186" for x in packages):
        subprocess.run(cargo + ["update", "-p", "libc@0.2.186", "--precise", "0.2.189", "--offline"] + config,
                       cwd=work, check=True)
binary = "wat_server"
if args.cancellation_probe:
    binary = "wat-cancellation-probe"
    probe = work / "crates/server/src/bin" / (binary + ".rs")
    probe.parent.mkdir(exist_ok=True)
    shutil.copyfile(Path(__file__).with_name("cancellation-probe.rs"), probe)
command = cargo + ["build", "-p", "wat_server", "--bin", binary, "--release", "--offline"]
if args.aros:
    command += ["--target", str(repo / "x86_64-aros.json"), "-Zbuild-std=std,panic_abort", "-Zjson-target-spec"]
subprocess.run(command + config, cwd=work, check=True, env=os.environ)
