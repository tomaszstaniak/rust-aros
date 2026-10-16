#!/usr/bin/env python3
"""Private finite TCP fixture; numeric loopback only, never a public listener."""
import argparse
import json
import socket
import threading
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--port', type=int, default=18761)
    parser.add_argument('--connections', type=int, default=83)
    args = parser.parse_args()
    errors = []
    workers = []
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
                    part = connection.recv(64 - len(request))
                    if not part:
                        break
                    request += part
                record(connection=number, request=request.decode('ascii'))
                if not request:
                    return  # cancellation/drop probe, no application request
                if request not in (b'delayed\n', b'fast\n'):
                    raise ValueError('unexpected request')
                if request == b'delayed\n':
                    time.sleep(.25)
                connection.sendall(b'R')
                connection.shutdown(socket.SHUT_WR)
        except Exception as error:
            with lock:
                errors.append(str(error))
            record(connection=number, error=str(error))

    with socket.socket() as listener:
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind(('127.0.0.1', args.port))
        listener.listen()
        deadline = time.monotonic() + 45
        record(ready=args.port, expected=args.connections)
        for number in range(args.connections):
            listener.settimeout(max(.001, deadline - time.monotonic()))
            connection, _ = listener.accept()
            worker = threading.Thread(target=serve, args=(connection, number))
            worker.start()
            workers.append(worker)
    for worker in workers:
        worker.join(timeout=4)
        if worker.is_alive():
            errors.append('fixture worker did not terminate')
    record(completed=len(workers), errors=errors)
    return int(bool(errors))


if __name__ == '__main__':
    raise SystemExit(main())
