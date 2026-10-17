#!/usr/bin/env python3
"""Bounded host observation of start and actual Shell-return receipts."""
import argparse
import json
import socket
import time
from pathlib import Path


def collect(listener, run_id, cases, timeout):
    if timeout <= 0 or not cases or len(set(cases)) != len(cases):
        raise ValueError('positive deadline and unique cases required')
    ready = time.monotonic()
    state = {name: {'status': 'waiting', 'deadline': ready + timeout} for name in cases}
    errors = []
    events = []
    while any(s['status'] in ('waiting', 'running') for s in state.values()):
        now = time.monotonic()
        for s in state.values():
            if s['status'] in ('waiting', 'running') and now >= s['deadline']:
                s['status'] = 'timeout'
        if not any(s['status'] in ('waiting', 'running') for s in state.values()):
            break
        listener.settimeout(min(.1, max(.001, min(s['deadline'] for s in state.values() if s['status'] in ('waiting', 'running')) - now)))
        try:
            peer, _ = listener.accept()
        except socket.timeout:
            continue
        with peer:
            peer.settimeout(min(1., max(.001, min(s['deadline'] for s in state.values() if s['status'] in ('waiting', 'running')) - time.monotonic())))
            data = b''
            try:
                while b'\n' not in data and len(data) < 1024:
                    part = peer.recv(1024 - len(data))
                    if not part:
                        break
                    data += part
                peer.sendall(b'OK\n')
            except (OSError, socket.timeout) as error:
                errors.append(str(error))
                continue
        now = time.monotonic()
        fields = data.decode('ascii', errors='replace').strip().split()
        events.append({'message': fields, 'host_seconds': now - ready})
        if len(fields) != 4 or fields[0] != run_id or fields[1] not in state or fields[2] not in ('start', 'end'):
            errors.append('unexpected or stale receipt'); continue
        _, name, phase, code = fields
        s = state[name]
        if now > s['deadline']:
            s['status'] = 'timeout'; errors.append('late receipt'); continue
        if phase == 'start' and s['status'] == 'waiting' and code == '0':
            s.update(status='running', started=now-ready, deadline=now+timeout)
        elif phase == 'end' and s['status'] == 'running':
            try:
                rc = int(code)
            except ValueError:
                errors.append('invalid return code'); continue
            s.update(status='passed' if rc == 0 else 'failed', returncode=rc, elapsed=now-ready-s['started'])
        else:
            errors.append('out-of-order receipt')
    for s in state.values():
        s.pop('deadline')
    return {'run_id': run_id, 'cases': state, 'errors': errors, 'events': events,
            'status': 'passed' if not errors and all(s['status'] == 'passed' for s in state.values()) else 'failed'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest', type=Path)
    parser.add_argument('variant', choices=['before', 'after'])
    parser.add_argument('output', type=Path)
    parser.add_argument('--timeout', type=float, default=60)
    parser.add_argument('--port', type=int, default=18791)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('output must be new')
    manifest = json.loads(args.manifest.read_text())
    identity = manifest['run_id'] + '-' + args.variant
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', args.port)); listener.listen()
        print(json.dumps({'ready': identity, 'port': args.port}), flush=True)
        result = collect(listener, identity, ['detach', 'parking', 'rayon'], args.timeout)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)


if __name__ == '__main__':
    main()
