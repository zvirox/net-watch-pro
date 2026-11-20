#!/usr/bin/env bash
# P8.11: verify a packaged install carries the eBPF collector without a
# separately installed object file.
#
#   --ci    container-safe smoke: the built binary embeds a real ELF object.
#   (default) real-host check: after `makepkg -si` and the one onboarding
#   pkexec, assert /status reports ebpf=attached. Run with sudo.
#
# [VERIFIED] for the host path is only honest when a human runs this on a real
# capable machine; CI containers cannot be treated as attach proof.
set -euo pipefail

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

MODE="${1:---host}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [ "$MODE" = "--ci" ]; then
  cd "$ROOT_DIR"
  [ -f target/netwatch_ebpf.o ] || fail "target/netwatch_ebpf.o missing (run cargo run -p xtask -- bpf)"
  cargo test -p netwatchd embedded_object_is_elf_when_present --quiet \
    || fail "netwatchd did not embed a valid ELF object"
  pass "CI smoke: netwatchd embeds the eBPF object"
  exit 0
fi

if [ "$MODE" != "--host" ]; then
  fail "unknown mode: $MODE (use --ci or --host)"
fi

command -v curl >/dev/null || fail "curl is required"
command -v python3 >/dev/null || fail "python3 is required"
command -v systemctl >/dev/null || fail "systemctl is required"
command -v netwatchd >/dev/null || fail "netwatchd package is not installed"

# The whole point of the change: no runtime object file may be needed.
if [ -e /usr/lib/netwatch-pro/netwatch_ebpf.o ]; then
  fail "unexpected runtime object at /usr/lib/netwatch-pro/netwatch_ebpf.o"
fi
pass "no runtime eBPF object file"

if [ "$(systemctl is-active netwatchd.service)" != "active" ]; then
  fail "netwatchd.service is not active (complete the onboarding pkexec first)"
fi
pass "daemon unit active"

SOCKET=/run/netwatch-pro/api.sock
STATUS="$(curl -fsS --unix-socket "$SOCKET" http://localhost/api/v1/status)" \
  || fail "cannot read /status over $SOCKET"

eval "$(printf '%s' "$STATUS" | python3 -c '
import json,sys
s=json.load(sys.stdin)
print("EBPF=%s" % s.get("ebpf",""))
print("RECORDING=%s" % ("true" if s.get("recording") else "false"))
')"

[ "$EBPF" = "attached" ] || fail "expected ebpf=attached, got $EBPF"
[ "$RECORDING" = "true" ] || fail "expected recording=true, got $RECORDING"
pass "status reports ebpf=attached and recording=true with no manual steps"

echo "HOST CHECK PASSED — record [VERIFIED] in CHECKLIST.md P8.11 with this output."
