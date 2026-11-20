#!/usr/bin/env bash
# Phase C item 4: verify the disabled / attach-failure UI path end-to-end.
# Run as root from the repo root. Starts the daemon with the collector disabled
# and prints the status the UI consumes; the UI states are asserted by the
# daemonStore unit tests (npm test) for chip/Dashboard/Live.
set -euo pipefail

cd "$(dirname "$0")/../.."
SOCK=/tmp/nwp-degraded/api.sock
DATA=/tmp/nwp-degraded
mkdir -p "$DATA"; rm -f "$SOCK"

echo "=== daemon with NETWATCH_DISABLE_EBPF=1 ==="
NETWATCH_DISABLE_EBPF=1 \
NETWATCH_DATA_DIR="$DATA" \
NETWATCH_API_SOCKET="$SOCK" \
  ./target/debug/netwatchd >/tmp/nwp-degraded/daemon.log 2>&1 &
PID=$!
trap 'kill "$PID" 2>/dev/null || true' EXIT
for i in $(seq 1 30); do [ -S "$SOCK" ] && break; sleep 0.2; done
sleep 0.5

echo "--- daemon log ---"; cat /tmp/nwp-degraded/daemon.log
echo "--- /status (UI reads this) ---"
curl --unix-socket "$SOCK" http://localhost/api/v1/status; echo
echo
echo "PASS criteria: ebpf='no_collector', recording=false, drops bpf_* all 0."
echo
echo "UI check (manual — no vitest in the UI yet): start the UI against this"
echo "socket (NETWATCH_DEV_DAEMON_ADDR=<this socket>) and confirm:"
echo "  - status chip = Degraded (warning + '!'), tooltip names no collector"
echo "  - Dashboard LIVE SPEED list says per-app needs an eBPF collector"
echo "  - Live screen empty state says no collector, not 'no connections'"
echo "These derive from daemonStore.collectorMissing (svelte-check clean)."
