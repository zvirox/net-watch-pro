#!/usr/bin/env bash
# P8.7: verify maintainer script stops/disables service on uninstall.
# Maintainer scripts are path-rewritten into a temp root so no host paths or
# systemd state are touched.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

fail() { echo "FAIL: $*" >&2; exit 1; }

make_root() {
  local base="$1"
  mkdir -p "$base/etc/systemd/system/netwatchd.service.d" \
           "$base/etc/netwatch-pro" \
           "$base/var/lib/netwatch-pro/data" \
           "$base/usr/share/applications"
  printf 'x' > "$base/etc/systemd/system/netwatchd.service.d/10-socket-group.conf"
  printf '1000\n' > "$base/etc/netwatch-pro/allowed-uids"
  printf 'db' > "$base/var/lib/netwatch-pro/data/netwatch.db"
}

rewrite() {
  local base="$1"
  sed -e "s|/etc/systemd/system/netwatchd.service.d|$base/etc/systemd/system/netwatchd.service.d|g" \
      -e "s|/etc/netwatch-pro|$base/etc/netwatch-pro|g" \
      -e "s|/var/lib/netwatch-pro|$base/var/lib/netwatch-pro|g" \
      -e "s|/usr/share/applications|$base/usr/share/applications|g" \
      -e "s|%{_sysconfdir}|$base/etc|g" \
      -e "s|%{nwp_datadir}|$base/var/lib/netwatch-pro|g" \
      -e 's|systemctl|true|g' \
      -e 's|update-desktop-database|true|g'
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# --- nfpm preremove test --------------------------------------------------
base="$TMP/nfpm-prerm"
make_root "$base"
rewrite "$base" < "$ROOT_DIR/packaging/scripts/preremove.sh" > "$TMP/nfpm-prerm.sh"
sh "$TMP/nfpm-prerm.sh"
echo "ok: nfpm preremove"

echo "P8.7 uninstall checks passed"
