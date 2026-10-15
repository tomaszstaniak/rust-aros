#!/usr/bin/env python3
"""Stage a real LSP session in RAM with fresh guest evidence names."""
import argparse
from pathlib import Path
import re
import shutil
p = argparse.ArgumentParser(description=__doc__)
p.add_argument("server", type=Path)
p.add_argument("client", type=Path)
p.add_argument("destination", type=Path)
p.add_argument("--prefix", required=True)
p.add_argument("--cancellation-probe", type=Path)
a = p.parse_args()
if not re.fullmatch(r"[A-Za-z0-9_-]{1,32}", a.prefix):
    p.error("prefix must be 1-32 ASCII letters, digits, underscores or hyphens")
if not all(x.is_file() for x in (a.server, a.client)):
    p.error("server and client must be regular files")
if a.cancellation_probe and not a.cancellation_probe.is_file():
    p.error("cancellation probe must be a regular file")
a.destination.mkdir()
for source, name in [(a.server, "wat-server"), (a.client, "lsp-check")]:
    shutil.copy2(source, a.destination / name)
    (a.destination / name).chmod(0o755)
(a.destination / "run-tests").write_text(f"""FailAt 21
Stack 1048576
Protect RAM:wl/wat-server +e
Protect RAM:wl/lsp-check +e
List RAM:wl >RESULTS:{a.prefix}-permissions.txt
Echo START >RESULTS:{a.prefix}-start.txt
RAM:wl/lsp-check RAM:wl/wat-server >RESULTS:{a.prefix}-transcript.txt
Echo $RC >RESULTS:{a.prefix}-rc.txt
Echo DONE >RESULTS:{a.prefix}-done.txt
""")
if a.cancellation_probe:
    probe = a.destination / "cancel-probe"
    shutil.copy2(a.cancellation_probe, probe)
    probe.chmod(0o755)
    script = a.destination / "run-tests"
    text = script.read_text()
    marker = f"Echo DONE >RESULTS:{a.prefix}-done.txt"
    text = text.replace(marker, f"""Protect RAM:wl/cancel-probe +e
RAM:wl/cancel-probe >RESULTS:{a.prefix}-cancellation.txt
Echo $RC >RESULTS:{a.prefix}-cancellation.rc
{marker}""")
    script.write_text(text)
