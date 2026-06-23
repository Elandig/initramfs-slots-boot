#!/usr/bin/env python3
"""Babysit a QEMU serial console over a unix socket.

It logs everything the guest prints, and whenever the slot machine's gate shows up
(`HIT 7 7 7 TO BOOT`) it types the recovery word to let the boot through - the same
thing a human would do. Exits 0 when the success pattern (a login prompt, by default)
appears, 1 on timeout.
"""

import argparse
import select
import socket
import sys
import time


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--socket", required=True, help="qemu -serial unix socket path")
    ap.add_argument("--log", required=True, help="file to tee the serial output into")
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--gate", default="TO BOOT", help="text that means the gate is up")
    ap.add_argument("--send", default="letmeboot\n", help="what to type at the gate")
    ap.add_argument("--success", default="login:", help="text that means we're through")
    args = ap.parse_args()

    deadline = time.time() + args.timeout

    # QEMU may not have created the listening socket yet.
    sock = None
    while time.time() < deadline:
        try:
            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.connect(args.socket)
            break
        except (FileNotFoundError, ConnectionRefusedError):
            sock.close()
            sock = None
            time.sleep(0.5)
    if sock is None:
        print("driver: never connected to the serial socket", file=sys.stderr)
        return 2

    sock.setblocking(False)
    gate = args.gate.encode()
    success = args.success.encode()
    send = args.send.encode()
    window = b""  # rolling tail we match patterns against
    last_bypass = 0.0
    gate_seen = False

    with open(args.log, "ab", buffering=0) as logf:
        while time.time() < deadline:
            ready, _, _ = select.select([sock], [], [], 1.0)
            if not ready:
                continue
            try:
                data = sock.recv(4096)
            except BlockingIOError:
                continue
            if not data:
                break
            logf.write(data)
            window = (window + data)[-8192:]

            if gate in window and time.time() - last_bypass > 8:
                try:
                    sock.sendall(send)
                except OSError:
                    pass
                last_bypass = time.time()
                gate_seen = True
                window = window.replace(gate, b"")
                print("driver: gate detected -> typed the recovery word", file=sys.stderr)

            if success in window:
                # Reaching login without ever seeing the gate means the hook didn't
                # gate the boot - that's a failure, not a pass.
                if gate_seen:
                    print("driver: success - the gate ran and the guest booted through", file=sys.stderr)
                    return 0
                print("driver: reached login but the GATE NEVER APPEARED", file=sys.stderr)
                return 3

    print("driver: timed out", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
