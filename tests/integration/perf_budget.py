#!/usr/bin/env python3
"""P10.2 perf-budget test: daemon CPU <= 3% and RSS <= 150 MB (p95).

Measures the daemon process under seeded SQLite + REST load. The eBPF packet
path is separately owner-verified in docs/spike-ebpf-report.md; this harness
covers the daemon task/DB path that CI can exercise without root.

Run: cargo build -p netwatchd && python3 tests/integration/perf_budget.py
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
CPU_LIMIT_PCT = 3.0
RSS_LIMIT_KIB = 150 * 1024


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def http_get(sock_path: str, path: str) -> dict:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(5)
        client.connect(sock_path)
        client.sendall(
            f"GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n".encode()
        )
        chunks = bytearray()
        while True:
            chunk = client.recv(65536)
            if not chunk:
                break
            chunks.extend(chunk)
    return json.loads(bytes(chunks).split(b"\r\n\r\n", 1)[1])


def sample(pid: int) -> tuple[float, int] | None:
    try:
        fields = (Path("/proc") / str(pid) / "stat").read_text().split()
        # utime(14), stime(15) are fields 14/15 => indexes 11/12 after comm.
        ticks = int(fields[11]) + int(fields[12])
        status = (Path("/proc") / str(pid) / "status").read_text()
        rss = next(
            int(line.split()[1])
            for line in status.splitlines()
            if line.startswith("VmRSS:")
        )
        return float(ticks), rss
    except (OSError, ValueError, StopIteration, IndexError):
        return None


def p95(values: list[float]) -> float:
    if not values:
        fail("no samples collected")
    ordered = sorted(values)
    return ordered[max(0, (95 * len(ordered) + 99) // 100 - 1)]


def main() -> int:
    if not DAEMON.is_file():
        fail(f"missing {DAEMON}; run `cargo build -p netwatchd` first")
    base = Path(tempfile.mkdtemp(prefix="nwp-perf-", dir=tempfile.gettempdir()))
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
                if http_get(sock_path, "/api/v1/health").get("status") == "ok":
                    break
            except (OSError, IndexError, ValueError):
                time.sleep(0.1)
        else:
            fail("daemon did not become healthy")

        # Seed a representative DB so queries do real work.
        now = int(time.time())
        db = sqlite3.connect(data / "netwatch.db")
        db.execute(
            "INSERT INTO apps (id, name, app_key, exe_path, first_seen_at, last_seen_at) "
            "VALUES (1, 'load', 'cgroup:1', 'load', ?1, ?1)",
            (now,),
        )
        rows = [
            (1, (now - i * 900) // 900 * 900, 500_000, 500_000, 8)
            for i in range(20_000)
        ]
        db.executemany(
            "INSERT OR IGNORE INTO usage_rollup_15m "
            "(app_id, bucket_start, bytes_up, bytes_down, connection_count) "
            "VALUES (?1, ?2, ?3, ?4, ?5)",
            rows,
        )
        db.commit()
        db.close()

        hz = os.sysconf("SC_CLK_TCK")
        previous = sample(daemon.pid)
        cpu_samples: list[float] = []
        rss_samples: list[int] = []
        started = time.monotonic()
        for _ in range(60):
            for path in (
                "/api/v1/usage/summary?range=7d",
                "/api/v1/apps?range=7d&limit=50",
                "/api/v1/connections/live",
            ):
                http_get(sock_path, path)
            time.sleep(0.05)
            current = sample(daemon.pid)
            if current and previous:
                elapsed = time.monotonic() - started
                delta_ticks = current[0] - previous[0]
                cpu_samples.append(delta_ticks / hz / max(elapsed, 0.001) * 100)
                rss_samples.append(current[1])
            previous = current
            started = time.monotonic()

        cpu = p95(cpu_samples)
        rss = p95([float(value) for value in rss_samples])
        print(f"INFO: samples={len(cpu_samples)} cpu_p95={cpu:.2f}% rss_p95={rss:.0f} KiB")
        if cpu > CPU_LIMIT_PCT:
            fail(f"CPU p95 {cpu:.2f}% exceeds {CPU_LIMIT_PCT}%")
        if rss > RSS_LIMIT_KIB:
            fail(f"RSS p95 {rss:.0f} KiB exceeds {RSS_LIMIT_KIB} KiB")
        print(f"PASS: perf budget cpu<={CPU_LIMIT_PCT}% rss<={RSS_LIMIT_KIB/1024:.0f} MB")
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
