#!/usr/bin/env python3
"""P10.3 seeded 1-year retention soak: DB stays under 1.5 GiB.

Creates the schema with the daemon, then seeds the volume profile from
ARCHITECTURE.md §9 (365 days x 40 apps x 96 fifteen-minute rollups plus
365 days x 40 apps x 20 endpoint-day rows) and checks the file size.
No year-long wait and no dependencies: Python stdlib only.

Run: cargo build -p netwatchd && python3 tests/integration/soak.py
"""

from __future__ import annotations

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
LIMIT_BYTES = int(1.5 * 1024**3)
DAYS, APPS, BUCKETS_PER_DAY, ENDPOINTS_PER_DAY = 365, 40, 96, 20
BUCKET = 900


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    if not DAEMON.is_file():
        fail(f"missing {DAEMON}; run `cargo build -p netwatchd` first")
    base = Path(tempfile.mkdtemp(prefix="nwp-soak-", dir=tempfile.gettempdir()))
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
        for _ in range(60):
            try:
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
                    client.settimeout(1)
                    client.connect(sock_path)
                    client.sendall(b"GET /api/v1/health HTTP/1.1\r\nHost: x\r\n\r\n")
                    if b'"status":"ok"' in client.recv(4096):
                        break
            except OSError:
                time.sleep(0.1)
        else:
            fail("daemon did not become healthy")
    finally:
        daemon.terminate()
        try:
            daemon.wait(timeout=10)
        except subprocess.TimeoutExpired:
            daemon.kill()
            daemon.wait(timeout=5)
        log.close()

    db_path = data / "netwatch.db"
    db = sqlite3.connect(db_path)
    db.execute("PRAGMA journal_mode=DELETE")
    db.execute("PRAGMA synchronous=OFF")
    now = int(time.time())
    day0 = (now - (DAYS - 1) * 86400) // 86400 * 86400
    db.execute(
        "INSERT INTO interfaces (id, name, kind) VALUES (2, 'wlan0', 'wifi')"
    )
    db.executemany(
        "INSERT INTO apps (id, name, app_key, exe_path, first_seen_at, last_seen_at) "
        "VALUES (?1, ?2, ?3, ?2, ?4, ?4)",
        [
            (app_id, f"app-{app_id}", f"cgroup:{app_id}", now)
            for app_id in range(1, APPS + 1)
        ],
    )
    db.commit()

    # 15-minute rollups: 365 * 40 * 96 = 1,401,600 rows.
    rollups: list[tuple[int, int, int, int, int]] = []
    inserted = 0
    for day in range(DAYS):
        day_start = day0 + day * 86400
        for slot in range(BUCKETS_PER_DAY):
            at = day_start + slot * BUCKET
            for app_id in range(1, APPS + 1):
                rollups.append((app_id, at, 120_000, 380_000, 4))
                if len(rollups) >= 50_000:
                    db.executemany(
                        "INSERT INTO usage_rollup_15m "
                        "(app_id, bucket_start, bytes_up, bytes_down, connection_count) "
                        "VALUES (?1, ?2, ?3, ?4, ?5)",
                        rollups,
                    )
                    inserted += len(rollups)
                    rollups.clear()
    if rollups:
        db.executemany(
            "INSERT INTO usage_rollup_15m "
            "(app_id, bucket_start, bytes_up, bytes_down, connection_count) "
            "VALUES (?1, ?2, ?3, ?4, ?5)",
            rollups,
        )
        inserted += len(rollups)
    db.commit()
    print(f"INFO: seeded {inserted} rollup rows")

    # Endpoint-day aggregates: 365 * 40 * 20 = 292,000 rows, top-N capped.
    endpoints: list[tuple[int, int, str, int, str, int, int, int, int, int]] = []
    for day in range(DAYS):
        day_start = day0 + day * 86400
        for app_id in range(1, APPS + 1):
            for endpoint in range(ENDPOINTS_PER_DAY):
                endpoints.append(
                    (
                        app_id,
                        day_start,
                        f"198.51.{day % 256}.{endpoint + 1}",
                        443,
                        "tcp",
                        60_000,
                        190_000,
                        3,
                        day_start,
                        day_start + 3600,
                    )
                )
    db.executemany(
        "INSERT INTO usage_endpoint_day "
        "(app_id, bucket_start, remote_ip, remote_port, protocol, bytes_up, "
        "bytes_down, connection_count, first_seen_at, last_seen_at) "
        "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        endpoints,
    )
    db.commit()
    db.execute("PRAGMA optimize")
    db.close()

    size = db_path.stat().st_size
    print(
        f"INFO: 1-year profile days={DAYS} apps={APPS} rollups={inserted} "
        f"endpoints={len(endpoints)} db_size={size / 1024**3:.3f} GiB"
    )
    if size >= LIMIT_BYTES:
        fail(f"DB {size / 1024**3:.3f} GiB exceeds 1.5 GiB")
    print("PASS: seeded 1-year retention stays under 1.5 GiB")
    shutil.rmtree(base, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
