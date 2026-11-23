#!/usr/bin/env python3
"""Stage 0 G4 / decision D9 — GSO upload-accounting spike (owner-run, root).

The spike report measured NIC tx_bytes roughly 42 B/packet above cgroup_skb
egress (57–66% attributed). D9 hypothesis: GSO makes the collector count one
large skb where the NIC counts several wire packets.

This script tests the hypothesis without changing collector code: it runs the
same sustained iperf3 upload twice, once with GSO enabled and once disabled,
and compares per-app attributed upload bytes from the daemon with NIC tx_bytes.
If disabling GSO closes the gap, the hypothesis is supported and the daemon
correction (skb->gso_segs) is justified. Otherwise D9 option 3 applies: document
the gap in the UI and rewrite the sum-check gate.

Run (root, on the CachyOS host, with an iperf3 server reachable):
    NETWATCH_IPERF_PEER=192.168.1.5 python3 tests/host/gso-spike.py
    NETWATCH_IPERF_PEER=... NETWATCH_SECONDS=60 NETWATCH_IFACE=wlan0 python3 tests/host/gso-spike.py

Requires: root, iperf3, ethtool, a live daemon with the collector attached.
"""

from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path

SOCKET = os.environ.get("NETWATCH_API_SOCKET", "/run/netwatch-pro/api.sock")
IFACE = os.environ.get("NETWATCH_IFACE", "")
PEER = os.environ.get("NETWATCH_IPERF_PEER", "")
SECONDS = int(os.environ.get("NETWATCH_SECONDS", "60"))


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def http_get(path: str):
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
    return json.loads(bytes(chunks).split(b"\r\n\r\n", 1)[1])


def default_interface() -> str:
    with open("/proc/net/route", encoding="ascii") as routes:
        for line in routes.readlines()[1:]:
            fields = line.split()
            if len(fields) >= 2 and fields[1] == "00000000":
                return fields[0]
    fail("no default route; set NETWATCH_IFACE")


def tx_bytes(iface: str) -> int:
    path = Path("/sys/class/net") / iface / "statistics" / "tx_bytes"
    if not path.is_file():
        fail(f"no tx_bytes for {iface}; set NETWATCH_IFACE")
    return int(path.read_text().strip())


def attributed_up() -> int:
    rows = http_get("/api/v1/apps?range=1h&limit=200&sort=usage_range")
    if not isinstance(rows, list):
        fail("unexpected /apps response")
    total = 0
    for row in rows:
        try:
            total += int(row["usage"]["range"]["up"])
        except (KeyError, TypeError, ValueError):
            continue
    return total


def set_gso(iface: str, enabled: bool) -> None:
    state = "on" if enabled else "off"
    subprocess.run(["ethtool", "-K", iface, "gso", state], check=True, capture_output=True)


def trial(iface: str, enabled: bool) -> dict:
    set_gso(iface, enabled)
    time.sleep(2)  # let in-flight skbs drain before sampling
    nic_before = tx_bytes(iface)
    up_before = attributed_up()
    result = subprocess.run(
        ["iperf3", "-c", PEER, "-t", str(SECONDS)], capture_output=True, text=True
    )
    # Give the daemon a moment to flush the last rollup before reading.
    time.sleep(2)
    nic_delta = tx_bytes(iface) - nic_before
    up_delta = attributed_up() - up_before
    if result.returncode != 0:
        fail(f"iperf3 failed: {result.stderr.strip()}")
    if nic_delta <= 0 or up_delta <= 0:
        fail(f"no upload recorded (nic={nic_delta}, attributed={up_delta})")
    return {
        "gso": enabled,
        "nic_bytes": nic_delta,
        "attributed_bytes": up_delta,
        "ratio_pct": 100.0 * up_delta / nic_delta,
    }


def main() -> int:
    if not PEER:
        fail("set NETWATCH_IPERF_PEER to an reachable iperf3 server")
    for tool in ("iperf3", "ethtool"):
        if shutil.which(tool) is None:
            fail(f"{tool} is not installed")
    if os.geteuid() != 0:
        fail("run as root: ethtool needs CAP_NET_ADMIN")

    iface = IFACE or default_interface()
    status = http_get("/api/v1/status")
    if status.get("ebpf") != "attached":
        fail("collector is not attached")

    original = subprocess.run(
        ["ethtool", "-k", iface], capture_output=True, text=True, check=True
    ).stdout
    gso_was_on = "gso: on" in original
    results: list[dict] = []
    try:
        for enabled in (True, False):
            results.append(trial(iface, enabled))
    finally:
        set_gso(iface, gso_was_on)

    for row in results:
        print(
            f"GSO {'on ' if row['gso'] else 'off'}: "
            f"attributed {row['attributed_bytes']:,} / NIC {row['nic_bytes']:,} "
            f"= {row['ratio_pct']:.1f}%"
        )
    on, off = results
    if off["ratio_pct"] - on["ratio_pct"] >= 5:
        print("Result: disabling GSO closes most of the gap — D9 option 2 (apply gso_segs correction).")
    else:
        print("Result: GSO is not the main cause — D9 option 3 (document the gap, rewrite the gate).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
