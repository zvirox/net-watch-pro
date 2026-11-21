#!/usr/bin/env bash
# Build the .rpm from release binaries. Run from anywhere:
#   ./packaging/rpm/build.sh [version]
# Version defaults to the latest git tag (minus 'v'); '~' is rewritten to '.'
# because RPM and Debian spell pre-release versions differently.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
  VERSION="$(git -C "$ROOT" describe --tags --abbrev=0 2>/dev/null | sed 's/^v//' || true)"
fi
if [[ -z "$VERSION" ]]; then
  echo "usage: $0 <version>   (e.g. 0.8.0~alpha.8)" >&2
  exit 2
fi
RPM_VERSION="$(printf '%s' "$VERSION" | sed 's/~/./g; s/-/./g')"

command -v rpmbuild >/dev/null 2>&1 || { echo "rpmbuild is required" >&2; exit 2; }

DAEMON="$ROOT/target/release/netwatchd"
APP="$ROOT/target/release/netwatch-app"
for bin in "$DAEMON" "$APP"; do
  [[ -x "$bin" ]] || { echo "missing release binary: $bin (run cargo build --release -p netwatchd -p netwatch-app)" >&2; exit 2; }
done

TOPDIR="$ROOT/target/rpmbuild"
SOURCES="$TOPDIR/SOURCES"
SPECS="$TOPDIR/SPECS"
mkdir -p "$SOURCES" "$SPECS" "$TOPDIR/BUILD" "$TOPDIR/RPMS" "$TOPDIR/SRPMS" "$TOPDIR/BUILDROOT"
OUT_DIR="$ROOT/target/rpm"
mkdir -p "$OUT_DIR"

install -m 0755 "$DAEMON" "$SOURCES/netwatchd"
install -m 0755 "$APP" "$SOURCES/netwatch-app"
install -m 0755 "$ROOT/packaging/polkit/enable-monitoring" "$SOURCES/enable-monitoring"
install -m 0644 "$ROOT/packaging/polkit/pro.netwatch.enable-monitoring.policy" "$SOURCES/pro.netwatch.enable-monitoring.policy"
install -m 0644 "$ROOT/packaging/systemd/netwatchd.service" "$SOURCES/netwatchd.service"
install -m 0644 "$ROOT/packaging/deb/net-watch-pro.desktop" "$SOURCES/net-watch-pro.desktop"
install -m 0644 "$ROOT/app/src-tauri/icons/icon@2x.png" "$SOURCES/net-watch-pro.png"
install -m 0644 "$ROOT/LICENSE" "$SOURCES/LICENSE"

sed "s|^%define nwp_version .*|%define nwp_version ${RPM_VERSION}|" \
  "$HERE/net-watch-pro.spec" > "$SPECS/net-watch-pro.spec"

rpmbuild -bb --define "_topdir $TOPDIR" "$SPECS/net-watch-pro.spec"

shopt -s nullglob
RPMS=("$TOPDIR"/RPMS/*/net-watch-pro-*.rpm)
(( ${#RPMS[@]} > 0 )) || { echo "rpmbuild produced no RPM" >&2; exit 1; }
cp "${RPMS[@]}" "$OUT_DIR/"
printf 'built: %s\n' "$OUT_DIR"
ls -1 "$OUT_DIR"
