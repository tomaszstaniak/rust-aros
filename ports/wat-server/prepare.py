#!/usr/bin/env python3
"""Prepare a pinned, disposable WAT workspace without editing upstream sources."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

PIN = "428f74da7fcb85d2fca9bd9828f5ff7434c19af8"
TREE_SHA256 = "334d85f511e539273cd8854e0a5bfbeb4ffb58721f66537c13fa25bc35ab4d4c"
LOCK_SHA256 = "e82670be946eaeeb414796a5a86092aa289e8f5d9d87a856b88a3b4aff0098fb"
parser = argparse.ArgumentParser(__doc__)
parser.add_argument("source", type=Path)
parser.add_argument("destination", type=Path)
parser.add_argument("--request-trace", action="store_true", help="apply optional diagnostic-only lifecycle trace")
parser.add_argument("--probe-checkpoint", action="store_true", help="synchronize one diagnostic Salsa checkpoint; implies request trace")
args = parser.parse_args()
repo = Path(__file__).resolve().parents[2]
source, dest = args.source.resolve(), args.destination.resolve()
if hashlib.sha256((source / "Cargo.lock").read_bytes()).hexdigest() != LOCK_SHA256:
    parser.error("upstream Cargo.lock does not match pinned source")
digest = hashlib.sha256()
files = sorted(p for p in source.rglob("*") if p.is_file()
               and not any(x in (".git", "target") for x in p.relative_to(source).parts))
for path in files:
    digest.update(path.relative_to(source).as_posix().encode() + b"\0" + hashlib.sha256(path.read_bytes()).digest())
if digest.hexdigest() != TREE_SHA256:
    parser.error("source tree does not match pinned upstream " + PIN)
if dest.is_relative_to(source):
    parser.error("destination must be outside the upstream source")
if dest.exists():
    parser.error("destination must not exist")
shutil.copytree(source, dest, ignore=shutil.ignore_patterns("target", ".git"))
subprocess.run(["patch", "-p1", "-i", str(Path(__file__).with_name("transport.patch").resolve())], cwd=dest, check=True)
if args.request_trace or args.probe_checkpoint:
    subprocess.run(["patch", "-p1", "-i", str(Path(__file__).with_name("request-trace.patch").resolve())], cwd=dest, check=True)
    shutil.copyfile(Path(__file__).with_name("request-trace.rs"), dest / "crates/server/src/request_trace.rs")
if args.probe_checkpoint:
    subprocess.run(["patch", "-p1", "-i", str(Path(__file__).with_name("probe-checkpoint.patch").resolve())], cwd=dest, check=True)
    for source_name, target_name in [("probe-checkpoint.rs", "probe_checkpoint.rs"), ("probe-gate.rs", "probe_gate.rs")]:
        shutil.copyfile(Path(__file__).with_name(source_name), dest / "crates/service/src" / target_name)
with (dest / "crates/server/Cargo.toml").open("a") as out:
    out.write('\n[dependencies.aros-lsp-transport]\npath = ' + json.dumps(str(repo / "transport")) + '\n')
print(dest)
