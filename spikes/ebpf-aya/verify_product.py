#!/usr/bin/env python3
"""Real-host verification of the Phase C product collector.

Connects to the product daemon's Unix socket, subscribes to net.rates +
live.apps, and prints frames for N seconds. Uses only the Python stdlib.

Usage:
  python3 verify_product.py /path/to/api.sock [seconds]
"""
import base64
import json
import os
import socket
import struct
import sys
import time
from urllib.parse import urlparse


def ws_connect(sock_path: str, timeout: float = 5.0):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(timeout)
    s.connect(sock_path)
    key = base64.b64encode(os.urandom(16)).decode()
    req = (
        "GET /api/v1/stream HTTP/1.1\r\n"
        f"Host: localhost\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        "Sec-WebSocket-Version: 13\r\n\r\n"
    )
    s.sendall(req.encode())
    buf = b""
    while b"\r\n\r\n" not in buf:
        chunk = s.recv(4096)
        if not chunk:
            raise RuntimeError("handshake closed")
        buf += chunk
    if b"101" not in buf.split(b"\r\n", 1)[0]:
        raise RuntimeError(f"handshake failed: {buf[:200]!r}")
    return s


def send_text(s, obj: dict):
    payload = json.dumps(obj).encode()
    mask = os.urandom(4)
    n = len(payload)
    head = bytearray([0x81])
    if n < 126:
        head.append(0x80 | n)
    elif n < 65536:
        head.append(0x80 | 126)
        head += struct.pack("!H", n)
    else:
        head.append(0x80 | 127)
        head += struct.pack("!Q", n)
    head += mask
    masked = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
    s.sendall(bytes(head) + masked)


def recv_frame(s):
    def exact(n):
        out = b""
        while len(out) < n:
            chunk = s.recv(n - len(out))
            if not chunk:
                raise EOFError()
            out += chunk
        return out

    b0, b1 = exact(2)
    opcode = b0 & 0x0F
    n = b1 & 0x7F
    if n == 126:
        n = struct.unpack("!H", exact(2))[0]
    elif n == 127:
        n = struct.unpack("!Q", exact(8))[0]
    if b1 & 0x80:
        mask = exact(4)
        data = bytes(b ^ mask[i % 4] for i, b in enumerate(exact(n)))
    else:
        data = exact(n)
    if opcode == 0x8:
        raise EOFError()
    if opcode != 0x1:
        return None
    return json.loads(data)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    sock_path = sys.argv[1]
    seconds = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    s = ws_connect(sock_path)
    send_text(s, {"op": "subscribe", "channel": "net.rates"})
    send_text(s, {"op": "subscribe", "channel": "live.apps"})
    print(f"subscribed to net.rates + live.apps for {seconds}s")
    end = time.time() + seconds
    apps_seen = 0
    s.settimeout(max(1.0, min(2.0, end - time.time())))
    while time.time() < end:
        try:
            frame = recv_frame(s)
        except socket.timeout:
            continue
        except EOFError:
            print("stream closed")
            break
        if not frame:
            continue
        channel = frame.get("channel")
        if channel == "live.apps":
            data = frame["data"]
            rows = data.get("apps", [])
            apps_seen += len(rows)
            motrix = [r for r in rows if "motrix" in r.get("name", "").lower()]
            print(
                f"live.apps ts={data.get('ts')} rows={len(rows)} "
                f"motrix={[(r['name'], r['down_bps'], r['confidence']) for r in motrix]} "
                f"unattributed={data.get('unattributed')}"
            )
        elif channel == "net.rates":
            d = frame["data"]
            print(f"net.rates down={d['down_bps']} up={d['up_bps']}")
    s.close()
    print(f"done; live.apps rows observed: {apps_seen}")


if __name__ == "__main__":
    main()
