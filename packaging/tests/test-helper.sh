#!/usr/bin/env bash
# Unit tests for /usr/libexec/netwatch-pro/enable-monitoring helper
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HELPER="$ROOT_DIR/packaging/polkit/enable-monitoring"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Create a mock systemctl so daemon-reload / enable don't hit the host
mkdir -p "$TMP/bin"
cat > "$TMP/bin/systemctl" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod +x "$TMP/bin/systemctl"
export PATH="$TMP/bin:$PATH"

# Test 1: Arguments rejected
set +e
"$HELPER" some-arg >/dev/null 2>"$TMP/err"
STATUS=$?
set -e
[ "$STATUS" -eq 2 ] || fail "expected exit 2 when arguments provided, got $STATUS"
grep -q "takes no arguments" "$TMP/err" || fail "expected 'takes no arguments' error"
pass "helper rejects arguments"

# Test 2: Missing PKEXEC_UID rejected
set +e
unset PKEXEC_UID
"$HELPER" >/dev/null 2>"$TMP/err"
STATUS=$?
set -e
[ "$STATUS" -eq 1 ] || fail "expected exit 1 when PKEXEC_UID missing, got $STATUS"
grep -q "PKEXEC_UID is missing" "$TMP/err" || fail "expected missing PKEXEC_UID error"
pass "helper rejects missing PKEXEC_UID"

# Test 3: Non-numeric PKEXEC_UID rejected
set +e
PKEXEC_UID="1000; rm -rf /" "$HELPER" >/dev/null 2>"$TMP/err"
STATUS=$?
set -e
[ "$STATUS" -eq 1 ] || fail "expected exit 1 for non-numeric PKEXEC_UID, got $STATUS"
grep -q "must be an unsigned integer" "$TMP/err" || fail "expected unsigned integer error"
pass "helper rejects non-numeric PKEXEC_UID"

# Test 4: PKEXEC_UID = 0 rejected
set +e
PKEXEC_UID=0 "$HELPER" >/dev/null 2>"$TMP/err"
STATUS=$?
set -e
[ "$STATUS" -eq 1 ] || fail "expected exit 1 for root PKEXEC_UID=0, got $STATUS"
grep -q "out of valid range" "$TMP/err" || fail "expected out of range error for 0"
pass "helper rejects root PKEXEC_UID 0"

# Test 5: PKEXEC_UID > 4294967294 rejected
set +e
PKEXEC_UID=4294967295 "$HELPER" >/dev/null 2>"$TMP/err"
STATUS=$?
set -e
[ "$STATUS" -eq 1 ] || fail "expected exit 1 for PKEXEC_UID > 4294967294, got $STATUS"
grep -q "out of valid range" "$TMP/err" || fail "expected out of range error for > 4294967294"
pass "helper rejects out-of-range PKEXEC_UID"

echo "ALL HELPER VALIDATION CHECKS PASSED"
