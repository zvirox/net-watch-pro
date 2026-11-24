#!/usr/bin/env bash
# P8.12: verify package upgrades restart an active netwatchd but never enable
# or start a stopped one. Maintainer scripts are path-rewritten into a temp
# root and systemctl is faked, so no host service state is touched.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
FAKE_LOG="$TMP/systemctl.log"
mkdir -p "$TMP/bin"

cat > "$TMP/bin/systemctl" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$FAKE_LOG"
if [ "$1" = "is-active" ]; then
  [ "${FAKE_ACTIVE:-0}" = "1" ] || exit 3
fi
exit 0
EOF
chmod +x "$TMP/bin/systemctl"
export PATH="$TMP/bin:$PATH"
export FAKE_LOG

make_root() {
  mkdir -p "$1/etc/systemd/system/netwatchd.service.d" \
           "$1/etc/netwatch-pro" \
           "$1/var/lib/netwatch-pro" \
           "$1/usr/share/applications"
}

rewrite() {
  sed -e "s|/etc/systemd/system/netwatchd.service.d|$1/etc/systemd/system/netwatchd.service.d|g" \
      -e "s|/etc/netwatch-pro|$1/etc/netwatch-pro|g" \
      -e "s|/var/lib/netwatch-pro|$1/var/lib/netwatch-pro|g" \
      -e "s|/usr/share/applications|$1/usr/share/applications|g" \
      -e "s|%{_sysconfdir}|$1/etc|g" \
      -e "s|%{nwp_datadir}|$1/var/lib/netwatch-pro|g" \
      -e 's|install -d -m 0750 -o root -g netwatch|install -d|g' \
      -e 's|groupadd --system netwatch|true|g' \
      -e 's|update-desktop-database|true|g'
}

check_restart() {
  local label="$1" expect="$2" found
  found="$(grep -c '^restart netwatchd.service$' "$FAKE_LOG" || true)"
  if [ "$expect" = yes ]; then
    [ "$found" = 1 ] || fail "$label: expected one restart, got $found"
  else
    [ "$found" = 0 ] || fail "$label: expected no restart, got $found"
  fi
  ! grep -q '^enable netwatchd.service$' "$FAKE_LOG" || fail "$label: must never enable the unit"
  ! grep -q '^start netwatchd.service$' "$FAKE_LOG" || fail "$label: must never start the unit"
  pass "$label"
}

# --- nfpm postinstall: restart active, no restart stopped -----------------
make_root "$TMP/nfpm"
rewrite "$TMP/nfpm" < "$ROOT_DIR/packaging/scripts/postinstall.sh" > "$TMP/nfpm-postinst"
: > "$FAKE_LOG"; FAKE_ACTIVE=1 bash "$TMP/nfpm-postinst"
check_restart "postinstall (active): restarts once" yes
: > "$FAKE_LOG"; FAKE_ACTIVE=0 bash "$TMP/nfpm-postinst"
check_restart "postinstall (stopped): no restart/start" no
