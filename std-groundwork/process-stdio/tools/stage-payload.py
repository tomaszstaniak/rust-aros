#!/usr/bin/env python3
"""Create a fresh VM-pool input directory, preserving guest exec capability."""
import argparse
from pathlib import Path
import re
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('destination', type=Path)
    parser.add_argument('--prefix', required=True)
    parser.add_argument('--include-buffered', action='store_true',
                        help='append stock-buffered LSP cases (known to block on older v11 PIPE:)')
    args = parser.parse_args()
    if not re.fullmatch(r'[A-Za-z0-9_-]{1,32}', args.prefix):
        parser.error('prefix must be 1-32 ASCII letters, digits, underscores or hyphens')
    if not args.binary.is_file():
        parser.error('binary must be a regular file')
    args.destination.mkdir()  # Never overwrite a previous payload.
    binary = args.destination / 'process-stdio'
    shutil.copy2(args.binary, binary)
    binary.chmod(0o755)
    lines = ['FailAt 21', 'Stack 1048576', 'Protect RAM:pr/process-stdio +e',
             f'List RAM:pr/process-stdio >RESULTS:{args.prefix}-permissions.txt']
    cases = ['status', 'args', 'spawn-error', 'exec-denied', 'wait', 'output', 'alloc-spawn',
             'kill', 'lsp-bytewise', 'lsp-invalid-bytewise']
    if args.include_buffered:
        cases.extend(['lsp', 'lsp-invalid'])
    for case in cases:
        lines.extend([f'Echo RUN-{case}',
                      f'RAM:pr/process-stdio --case {case} --report RESULTS:{args.prefix}-{case}.txt >RESULTS:{args.prefix}-{case}.out',
                      f'Echo $RC >RESULTS:{args.prefix}-{case}.rc'])
    lines.append(f'Echo SCRIPT-RETURNED >RESULTS:{args.prefix}-done.txt')
    (args.destination / 'run-tests').write_text('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
