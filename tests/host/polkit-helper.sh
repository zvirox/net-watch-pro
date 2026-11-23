#!/usr/bin/env bash
# Stage 0 G7: prove the polkit helper fails loudly on privileged-step failures.
# Runs as an unprivileged user against temporary path overrides and stubbed
# usermod/systemctl/getent commands. No system file is touched.
#
# Run: bash tests/host/polkit-helper.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HELPER="$ROOT/packaging/polkit/enable-monitoring"
BASE="$(mktemp -d "${TMPDIR:-/tmp}/nwp-polkit-XXXXXX")"
cleanup() {
  find "$BASE" -type d -exec chmod 0700 {} + 2>/dev/null || true
  rm -rf "$BASE"
}
trap cleanup EXIT

pass=0
fail=0
check() {
  if [ "$1" -eq 0 ]; then
    printf 'PASS %s\n' "$2"
    pass=$((pass + 1))
  else
    printf 'FAIL %s\n' "$2" >&2
    fail=$((fail + 1))
  fi
}
assert_eq() {
  if [ "$1" = "$2" ]; then check 0 "$3"; else check 1 "$3 (expected $2, got $1)"; fi
}

make_stubs() {
  mkdir -p "$BASE/bin"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$BASE/bin/usermod"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$BASE/bin/systemctl"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$BASE/bin/getent"
  chmod +x "$BASE/bin"/*
}

run_helper() {
  PKEXEC_UID="$(id -u)" \
  PATH="$BASE/bin:$PATH" \
  NETWATCH_ETC_DIR="$BASE/etc" \
  NETWATCH_SYSTEMD_DIR="$BASE/systemd" \
    bash "$HELPER" >"$BASE/out" 2>"$BASE/err"
}

# --- 1. Happy path: allowlist + drop-in are written, exit 0 -------------------
make_stubs
mkdir -p "$BASE/etc" "$BASE/systemd"
run_helper
check "$?" 'happy path exits 0'
assert_eq "$(grep -c "$(id -u)" "$BASE/etc/allowed-uids" 2>/dev/null)" 1 'allowlist contains caller UID'
assert_eq "$(grep -c "NETWATCH_API_GID=$(id -g)" "$BASE/systemd/netwatchd.service.d/10-socket-group.conf" 2>/dev/null)" 1 'drop-in contains caller GID'

# --- 2. usermod failure: non-zero exit with a diagnostic ----------------------
printf '#!/usr/bin/env bash\necho "stub: usermod failed" >&2\nexit 1\n' > "$BASE/bin/usermod"
chmod +x "$BASE/bin/usermod"
run_helper
rc=$?
if [ "$rc" -ne 0 ] && grep -q 'cannot add' "$BASE/err"; then ok=0; else ok=1; fi
check "$ok" 'usermod failure exits non-zero with a diagnostic'

# --- 3. Read-only drop-in dir: non-zero exit with a diagnostic ----------------
make_stubs
rm -f "$BASE/systemd/netwatchd.service.d/10-socket-group.conf"
chmod 0500 "$BASE/systemd/netwatchd.service.d"
run_helper
rc=$?
if [ "$rc" -ne 0 ] && grep -q 'cannot write' "$BASE/err"; then ok=0; else ok=1; fi
check "$ok" 'read-only drop-in dir exits non-zero with a diagnostic'

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
