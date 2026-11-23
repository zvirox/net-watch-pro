#!/usr/bin/env python3
"""Stage 0 G3 — host attribution harness (owner-run, requires root + live eBPF).

Complements tests/accuracy/accuracy.py, which runs a debug daemon with the
collector disabled against seeded data. This script measures a *real* daemon
with the eBPF collector attached and compares per-app accounting with the
kernel's interface counters while traffic flows.

It prints attributed and unattributed percentages alongside daemon CPU and RSS.
It does not assert an accuracy threshold: the seeded 95% gate is accuracy.py,
and the host upload gap is tracked separately as G4/D9. A successful run means
the numbers were collected and the collector was actually attached.

Run (root, on the CachyOS host, with traffic generated during the sample):
    python3 tests/host/attribution.py
    NETWATCH_IFACE=wlan0 NETWATCH_SAMPLE_SECONDS=60 python3 tests/host/attribution.py
"""

from __future__ import annotations

import json
import os
import socket
import sys
import time
from pathlib import Path

SOCKET = os.environ.get("NETWATCH_API_SOCKET", "/run/netwatch-pro/api.sock")
IFACE = os.environ.get("NETWATCH_IFACE", "")
SECONDS = int(os.environ.get("NETWATCH_SAMPLE_SECONDS", "60"))


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def http_get(path: str) -> dict:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(5)
        client.connect(SOCKET)
        request = f"GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        client.sendall(request.encode())
        chunks = bytearray()
        while True:
            chunk = client.recv(65536)
            if not chunk:
                break
            chunks.extend(chunk)
    body = bytes(chunks).split(b"\r\n\r\n", 1)[1]
    return json.loads(body)


def default_interface() -> str:
    """Pick the interface used for the default route from /proc/net/route."""
    with open("/proc/net/route", encoding="ascii") as routes:
        for line in routes.readlines()[1:]:
            fields = line.split()
            if len(fields) >= 2 and fields[1] == "00000000":
                return fields[0]
    fail("no default route in /proc/net/route; set NETWATCH_IFACE")


def nic_bytes(iface: str) -> tuple[int, int]:
    base = Path("/sys/class/net") / iface / "statistics"
    if not base.is_dir():
        fail(f"interface {iface} not found; set NETWATCH_IFACE")
    return (
        int((base / "rx_bytes").read_text().strip()),
        int((base / "tx_bytes").read_text().strip()),
    )


def totals() -> dict[str, int]:
    payload = http_get("/api/v1/usage/summary?range=1h")
    try:
        block = payload["range"]["totals"]
        return {key: int(block[key]) for key in ("total", "attributed", "unattributed")}
    except (KeyError, TypeError, ValueError) as err:
        fail(f"unexpected /usage/summary shape: {err}")


def daemon_pid() -> int:
    for proc in Path("/proc").glob("[0-9]*/comm"):
        try:
            if proc.read_text().strip() == "netwatchd":
                return int(proc.parent.name)
        except OSError:
            continue
    fail("netwatchd process not found")


def cpu_and_rss(pid: int) -> tuple[float, int]:
    fields = Path(f"/proc/{pid}/stat").read_text().split()
    ticks = int(fields[13]) + int(fields[14])
    rss_kib = 0
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith("VmRSS:"):
            rss_kib = int(line.split()[1])
            break
    return ticks / os.sysconf("SC_CLK_TCK"), rss_kib


def percent(part: int, whole: int) -> float:
    return 100.0 * part / whole if whole else 0.0


def main() -> int:
    iface = IFACE or default_interface()
    status = http_get("/api/v1/status")
    if status.get("ebpf") != "attached" or not status.get("recording"):
        fail(f"collector not attached/recording: {status.get('ebpf')}/{status.get('recording')}")
    seen = status.get("drops", {}).get("bpf_packets_seen", 0)
    if not seen:
        fail("bpf_packets_seen is 0; no packets reached the collector")

    pid = daemon_pid()
    nic_before = nic_bytes(iface)
    sum_before = totals()
    cpu_before, _ = cpu_and_rss(pid)
    wall_before = time.monotonic()

    print(f"Sampling {iface} for {SECONDS}s — generate traffic now", flush=True)
    time.sleep(SECONDS)

    wall = time.monotonic() - wall_before
    nic_after = nic_bytes(iface)
    sum_after = totals()
    cpu_after, rss_kib = cpu_and_rss(pid)

    nic_delta = (nic_after[0] - nic_before[0]) + (nic_after[1] - nic_before[1])
    if nic_delta <= 0:
        fail("interface counters did not advance; no traffic was captured")
    attributed = sum_after["attributed"] - sum_before["attributed"]
    unattributed = sum_after["unattributed"] - sum_before["unattributed"]
    if attributed + unattributed <= 0:
        fail("daemon rollups did not advance; check range=1h and recording state")

    cpu_pct = 100.0 * (cpu_after - cpu_before) / wall if wall else 0.0
    print(f"Interface:        {iface}")
    print(f"Kernel bytes:     {nic_delta:,}")
    print(f"Attributed bytes: {attributed:,}  ({percent(attributed, nic_delta):.1f}% of NIC)")
    print(f"Unattributed:     {unattributed:,}  ({percent(unattributed, nic_delta):.1f}% of NIC)")
    print(f"Daemon CPU:       {cpu_pct:.2f}% over {wall:.1f}s")
    print(f"Daemon RSS:       {rss_kib:,} KiB")
    print(
        f"Collector:        ebpf={status.get('ebpf')}, "
        f"packets_seen={status.get('drops', {}).get('bpf_packets_seen', 0):,}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
