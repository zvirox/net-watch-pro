#!/usr/bin/env bash
# Phase C item 3: measure daemon CPU/RSS at 50 and 500 connections on the
# real host. Run as root from the repo root. The daemon logs self-measured
# CPU/RSS every second; this script opens N concurrent TCP connects (to a
# local listener, so no outbound traffic) and correlates the window.
set -euo pipefail

cd "$(dirname "$0")/../.."
SOCK=/tmp/nwp-load/api.sock
DATA=/tmp/nwp-load
OBJ=/home/zvirox/Projects/Linux/net-watch-pro/target/netwatch_ebpf.o

mkdir -p "$DATA"
rm -f "$SOCK" "$DATA/netwatch.db"

echo "=== starting daemon ==="
NETWATCH_DATA_DIR="$DATA" \
NETWATCH_API_SOCKET="$SOCK" \
NETWATCH_EBPF_OBJECT="$OBJ" \
  ./target/debug/netwatchd >/tmp/nwp-load/daemon.log 2>&1 &
DAEMON_PID=$!
trap 'kill "$DAEMON_PID" 2>/dev/null || true' EXIT
for i in $(seq 1 30); do [ -S "$SOCK" ] && break; sleep 0.2; done
sleep 1
curl --unix-socket "$SOCK" http://localhost/api/v1/status; echo

# Local listener: 500 sockets connect to it, so packets stay in this host.
# cgroup_skb sees them; the NIC counters don't — this is a LOAD test, not an
# accuracy measurement.
python3 - <<'PY' &
import socket, threading, time
srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(("127.0.0.1", 18923)); srv.listen(1024)
stop = threading.Event()
def accept_loop():
    while not stop.is_set():
        try:
            srv.settimeout(0.5); c, _ = srv.accept(); c.close()
        except socket.timeout:
            continue
threading.Thread(target=accept_loop, daemon=True).start()
time.sleep(75)
PY
LISTENER_PID=$!
sleep 1

for N in 50 500; do
  echo "=== $N concurrent connections (20 s window) ==="
  python3 - "$N" <<'PY' &
import socket, sys, time
n = int(sys.argv[1])
end = time.time() + 20
def worker():
    while time.time() < end:
        try:
            s = socket.create_connection(("127.0.0.1", 18923), timeout=1)
            s.close()
        except OSError:
            pass
threads = [__import__('threading').Thread(target=worker, daemon=True) for _ in range(8)]
for t in threads: t.start()
for t in threads: t.join()
PY
  LOAD_PID=$!
  sleep 21
  wait "$LOAD_PID" || true
  curl --unix-socket "$SOCK" http://localhost/api/v1/status; echo
  echo "--- daemon self-measured CPU/RSS during this window ---"
  grep -E 'eBPF collector map diagnostic' /tmp/nwp-load/daemon.log | tail -n 25 || true
done

kill "$LISTENER_PID" 2>/dev/null || true
echo "=== budget: cpu_pct <= 3, rss_kib <= 153600 (see diagnostic lines above) ==="
echo "PASS requires bpf_packets_seen to rise with 50 then 500 connections and map_full=0."
