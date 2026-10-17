#!/usr/bin/env python3
"""Finite controlled async TCP fixture, numeric loopback and private refusal port."""
import argparse
import json
import socket
import threading
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--port', type=int, default=18761)
    parser.add_argument('--refused-port', type=int, default=18762)
    parser.add_argument('--connections', type=int, default=88)
    parser.add_argument('--deadline', type=float, default=60)
    parser.add_argument('--empty-connections', type=int, default=0)
    args = parser.parse_args()
    if args.deadline <= 0:
        parser.error('deadline must be positive')
    if not 0 <= args.empty_connections <= args.connections:
        parser.error('empty-connections must be within total connection count')
    workers, errors = [], []
    empty_count = [0]
    timer_completed = threading.Event()
    lock = threading.Lock()

    def record(**entry):
        with lock:
            print(json.dumps(entry), flush=True)

    def serve(connection, number):
        try:
            with connection:
                connection.settimeout(3)
                request = b''
                while not request.endswith(b'\n') and len(request) < 64:
                    part = connection.recv(64-len(request))
                    if not part:
                        break
                    request += part
                record(connection=number, request=request.decode('ascii'))
                if request == b'':
                    with lock:
                        empty_count[0] += 1
                        allowed = empty_count[0] <= args.empty_connections
                    if not allowed:
                        raise ValueError('unexpected empty connection')
                    return
                if request == b'done\n':
                    return
                if request == b'cancel\n':
                    if connection.recv(1) != b'':
                        raise ValueError('cancelled client sent additional data')
                    record(connection=number, cancelled=True)
                    return
                if request == b'timer-done\n':
                    timer_completed.set()
                elif request == b'timer-race\n':
                    if not timer_completed.wait(timeout=3):
                        raise ValueError('timer completion control did not arrive')
                elif request not in (b'delayed\n', b'fast\n'):
                    raise ValueError('unexpected request')
                if request == b'delayed\n':
                    time.sleep(.25)
                connection.sendall(b'R')
                connection.shutdown(socket.SHUT_WR)
        except Exception as error:
            with lock:
                errors.append(str(error))
            record(connection=number, error=str(error))

    # On macOS a bound non-listening socket drops SYNs rather than refusing.
    # Check availability, close the port, then verify actual refusal at each end.
    with socket.socket() as reservation:
        reservation.bind(('127.0.0.1', args.refused_port))

    def verify_refusal():
        with socket.socket() as client:
            client.settimeout(.3)
            try:
                client.connect(('127.0.0.1', args.refused_port))
            except ConnectionRefusedError:
                return
            raise RuntimeError('refusal endpoint did not reject the connection')

    verify_refusal()
    with socket.socket() as listener:
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind(('127.0.0.1', args.port))
        listener.listen()
        record(ready=args.port, refused=args.refused_port, expected=args.connections)
        deadline = time.monotonic() + args.deadline
        for number in range(args.connections):
            listener.settimeout(max(.001, deadline-time.monotonic()))
            connection, _ = listener.accept()
            worker = threading.Thread(target=serve, args=(connection,number))
            worker.start()
            workers.append(worker)
        # The final done request follows the guest refusal assertions.
        for worker in workers:
            worker.join(timeout=4)
            if worker.is_alive():
                errors.append('fixture worker did not terminate')
    verify_refusal()
    if empty_count[0] != args.empty_connections:
        errors.append('empty connection count mismatch')
    record(completed=len(workers), errors=errors)
    return int(bool(errors))


if __name__ == '__main__':
    raise SystemExit(main())
