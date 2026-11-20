#!/usr/bin/env bash
# Real-host verification of the Phase C collector. Run as root from the repo
# root. Starts the built daemon in the background, waits for the socket,
# prints status, then runs the live.apps verifier. Stops the daemon on exit.
set -euo pipefail

cd "$(dirname "$0")/../.."   # repo root
SOCK=/tmp/nwp-verify-run/api.sock
DATA=/tmp/nwp-verify-run
OBJ=/home/zvirox/Projects/Linux/net-watch-pro/target/netwatch_ebpf.o

mkdir -p "$DATA"
rm -f "$SOCK"

echo "=== starting daemon (object: $OBJ) ==="
NETWATCH_DATA_DIR="$DATA" \
NETWATCH_API_SOCKET="$SOCK" \
NETWATCH_EBPF_OBJECT="$OBJ" \
  ./target/debug/netwatchd >/tmp/nwp-verify-run/daemon.log 2>&1 &
DAEMON_PID=$!
trap 'kill "$DAEMON_PID" 2>/dev/null || true' EXIT

for i in $(seq 1 30); do
  [ -S "$SOCK" ] && break
  sleep 0.2
done
sleep 0.5

echo "=== daemon log ==="
cat /tmp/nwp-verify-run/daemon.log
echo "=== status ==="
curl --unix-socket "$SOCK" http://localhost/api/v1/status
echo
echo "=== bpftool: our programs ==="
bpftool cgroup tree | grep -E 'cgroup_inet_(ingress|egress)' | grep -E ' (ingress|egress)[[:space:]]*$' || true
echo
echo "=== live.apps verifier (start Motrix download first!) ==="
python3 spikes/ebpf-aya/verify_product.py "$SOCK" 20
echo "=== rollups after ~60s checkpoint ==="
sqlite3 "$DATA/netwatch.db" \
  "SELECT a.name, a.app_key, SUM(r.bytes_down), SUM(r.bytes_up)
   FROM usage_rollup_15m r JOIN apps a ON a.id=r.app_id GROUP BY a.name, a.app_key;"
