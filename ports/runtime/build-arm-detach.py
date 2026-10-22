#!/usr/bin/env python3
"""Build the existing detach probe against the native Pi SDK, without SDK edits."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile

SOURCE_REV = '7c24ef2ef03cd0ef1cbf2bcbad4227f7d62d2652'
ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('aros-source', 'cc', 'sdk', 'out'):
        p.add_argument('--' + name, type=Path, required=True)
    a = p.parse_args()
    for name in ('aros_source', 'cc', 'sdk', 'out'):
        setattr(a, name, getattr(a, name).absolute())
    a.out.mkdir(parents=True, exist_ok=False)
    archive = subprocess.check_output(['git', '-C', a.aros_source, 'archive', SOURCE_REV,
                                       'compiler/pthread'], timeout=60)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(a.out / 'source', filter='data')
    source = a.out / 'source/compiler/pthread'
    # Read-only diagnostic fields: the installed archive's GetThreadInfo has
    # stride 0x390, task at 0x28; detach reads detached at 0x384. Preserve the
    # pinned complete header and require its layout under the *ARM SDK*.
    contract = '\n#include <stddef.h>\n'
    for expression, expected in [('sizeof(ThreadInfo)', 912),
                                 ('offsetof(ThreadInfo, finished)', 32),
                                 ('offsetof(ThreadInfo, task)', 40),
                                 ('offsetof(ThreadInfo, detached)', 900)]:
        contract += f'_Static_assert({expression} == {expected}, "Pi SDK layout: {expression}");\n'
    probe = a.out / 'detach.c'
    probe.write_text((ROOT / 'ports/runtime/detach.c').read_text().replace(
        '#include "pthread_intern.h"', '#include "pthread_intern.h"' + contract))
    command = [a.cc, '--sysroot=' + str(a.sdk), '-O2', '-I', source,
               '-DRUNTIME_RUN_ID="arm-detach-baseline"', probe, '-lpthread',
               '-Wl,--no-gc-sections', '-o', a.out / 'detach']
    subprocess.run(list(map(str, command)), check=True, timeout=120)
    data = (a.out / 'detach').read_bytes()
    if data[:9] != bytes.fromhex('7f454c460201010f01') or int.from_bytes(data[18:20], 'little') != 183:
        raise ValueError('require native AROS ELF64 AArch64 ABIv1')
    metadata = dict(profile='aarch64-v1-raspi', source_revision=SOURCE_REV,
                    source_archive_sha256=hashlib.sha256(archive).hexdigest(),
                    sdk_pthread_sha256=sha(a.sdk / 'lib/libpthread.a'),
                    compiler=subprocess.check_output([a.cc, '--version'], text=True),
                    command=list(map(str, command)), probe_sha256=sha(probe),
                    artifact_sha256=sha(a.out / 'detach'), runtime_acceptance=False)
    (a.out / 'build.json').write_text(json.dumps(metadata, indent=2) + '\n')


if __name__ == '__main__':
    main()
