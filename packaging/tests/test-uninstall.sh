#!/usr/bin/env bash
# P8.7: verify all three package formats remove service residue on uninstall,
# delete data by default, and honour the opt-in keep-data marker.
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

check_default() {
  local base="$1"
  [ ! -e "$base/etc/systemd/system/netwatchd.service.d/10-socket-group.conf" ] || fail "drop-in left behind"
  [ ! -e "$base/etc/netwatch-pro/allowed-uids" ] || fail "allowed-uids left behind"
  [ ! -e "$base/etc/netwatch-pro" ] || fail "config dir left behind"
  [ ! -e "$base/var/lib/netwatch-pro" ] || fail "data kept without marker"
}

check_optin() {
  local base="$1"
  [ ! -e "$base/etc/systemd/system/netwatchd.service.d/10-socket-group.conf" ] || fail "drop-in left behind (opt-in)"
  [ ! -e "$base/etc/netwatch-pro/allowed-uids" ] || fail "allowed-uids left behind (opt-in)"
  [ -f "$base/var/lib/netwatch-pro/data/netwatch.db" ] || fail "data removed despite keep-data marker"
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# --- deb: postrm remove, with and without marker --------------------------
for mode in default optin; do
  base="$TMP/deb-$mode"
  make_root "$base"
  [ "$mode" = optin ] && : > "$base/etc/netwatch-pro/keep-data"
  rewrite "$base" < "$ROOT_DIR/packaging/deb/postrm" > "$TMP/deb-postrm.sh"
  sh "$TMP/deb-postrm.sh" remove
  if [ "$mode" = optin ]; then check_optin "$base"; else check_default "$base"; fi
  echo "ok: deb $mode"
done

# --- rpm: %postun scriptlet, with and without marker ----------------------
awk '/^%postun$/{flag=1;next}/^%files$/{flag=0}flag' \
  "$ROOT_DIR/packaging/rpm/net-watch-pro.spec" > "$TMP/rpm-postun.raw"
for mode in default optin; do
  base="$TMP/rpm-$mode"
  make_root "$base"
  [ "$mode" = optin ] && : > "$base/etc/netwatch-pro/keep-data"
  rewrite "$base" < "$TMP/rpm-postun.raw" > "$TMP/rpm-postun.sh"
  sh "$TMP/rpm-postun.sh" 0
  if [ "$mode" = optin ]; then check_optin "$base"; else check_default "$base"; fi
  echo "ok: rpm $mode"
done

# --- arch: post_remove() from the .install hook ---------------------------
for mode in default optin; do
  base="$TMP/arch-$mode"
  make_root "$base"
  [ "$mode" = optin ] && : > "$base/etc/netwatch-pro/keep-data"
  rewrite "$base" < "$ROOT_DIR/packaging/arch/net-watch-pro-git.install" > "$TMP/arch-install.sh"
  bash -c "source '$TMP/arch-install.sh'; post_remove"
  if [ "$mode" = optin ]; then check_optin "$base"; else check_default "$base"; fi
  echo "ok: arch $mode"
done

echo "P8.7 uninstall checks passed"
