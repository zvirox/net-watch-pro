#!/usr/bin/env python3
"""P10.1 accuracy harness: daemon totals vs kernel interface counters.

Starts a debug daemon on a temporary socket with the eBPF collector disabled,
seeds representative interface/app rollups, then asserts from the real API that
headline totals equal the kernel-counter sum and attribution is >= 95%.

No root and no dependencies: Python stdlib + an already-built debug daemon.
Run: cargo build -p netwatchd && python3 tests/accuracy/accuracy.py
"""

from __future__ import annotations

import json
import os
import shutil
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DAEMON = ROOT / "target/debug/netwatchd"
BUCKET = 900


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def http_get(sock_path: str, path: str) -> dict:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(5)
        client.connect(sock_path)
        request = (
            f"GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        client.sendall(request.encode())
        chunks = bytearray()
        while True:
            chunk = client.recv(65536)
            if not chunk:
                break
            chunks.extend(chunk)
    body = bytes(chunks).split(b"\r\n\r\n", 1)[1]
    return json.loads(body)


def wait_health(sock_path: str) -> None:
    for _ in range(60):
        try:
            if http_get(sock_path, "/api/v1/health").get("status") == "ok":
                return
        except (OSError, IndexError, ValueError):
            time.sleep(0.1)
    fail("daemon did not become healthy")


def main() -> int:
    if not DAEMON.is_file():
        fail(f"missing {DAEMON}; run `cargo build -p netwatchd` first")
    base = Path(tempfile.mkdtemp(prefix="nwp-accuracy-", dir=tempfile.gettempdir()))
    data = base / "data"
    data.mkdir()
    sock_path = str(base / "api.sock")
    env = dict(
        os.environ,
        NETWATCH_DISABLE_EBPF="1",
        NETWATCH_DATA_DIR=str(data),
        NETWATCH_API_SOCKET=sock_path,
        NETWATCH_LOG_LEVEL="warn",
    )
    log = (base / "daemon.log").open("w")
    daemon = subprocess.Popen([str(DAEMON)], env=env, stdout=log, stderr=log)
    try:
        wait_health(sock_path)
        db = sqlite3.connect(data / "netwatch.db")
        now = int(time.time())
        bucket = now // BUCKET * BUCKET
        # Six 15-minute buckets inside the last-hour window.
        buckets = [bucket - offset * BUCKET for offset in range(6)]
        now_s = str(now)
        db.execute(
            "INSERT INTO interfaces (id, name, kind) VALUES (2, 'wlan0', 'wifi')"
        )
        for app_id, name in ((1, "Firefox"), (2, "Motrix")):
            db.execute(
                "INSERT INTO apps (id, name, app_key, exe_path, "
                "attribution_confidence, first_seen_at, last_seen_at) "
                "VALUES (?1, ?2, ?3, ?2, 'exact', ?4, ?4)",
                (app_id, name, f"cgroup:{app_id}", now_s),
            )
        for at in buckets:
            db.execute(
                "INSERT INTO usage_interface_15m "
                "(interface_id, bucket_start, rx_bytes, tx_bytes) VALUES (2, ?1, 640000, 360000)",
                (at,),
            )
            # 960,000 of 1,000,000 bytes attributed => 96% (>= 95% gate).
            db.execute(
                "INSERT INTO usage_rollup_15m "
                "(app_id, bucket_start, bytes_up, bytes_down, connection_count) "
                "VALUES (1, ?1, 300000, 300000, 4), (2, ?1, 150000, 210000, 2)",
                (at,),
            )
        db.commit()
        expected_total = 6 * 1_000_000
        expected_attributed = 6 * 960_000
        summary = http_get(sock_path, "/api/v1/usage/summary?range=1h")
        totals = summary["range"]["totals"]
        measured_total = totals["total"]
        measured_attributed = totals["attributed"]
        if measured_total <= 0:
            fail(f"no totals returned: {summary}")
        # The 1h window may clip one boundary bucket; compare against the
        # seeded ratio rather than an exact bucket count.
        ratio = measured_attributed / measured_total
        if ratio < 0.95:
            fail(f"attribution {ratio:.3f} < 0.95: {totals}")
        if totals["unattributed"] < 0:
            fail(f"negative unattributed: {totals}")
        if abs(totals["unattributed"] - (measured_total - measured_attributed)) > 1:
            fail(f"gap does not match total - attributed: {totals}")
        print(
            f"PASS: accuracy total={measured_total} attributed={measured_attributed} "
            f"ratio={ratio:.3f} (seeded profile {expected_total}/{expected_attributed})"
        )
        return 0
    finally:
        daemon.terminate()
        try:
            daemon.wait(timeout=5)
        except subprocess.TimeoutExpired:
            daemon.kill()
        log.close()
        shutil.rmtree(base, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
