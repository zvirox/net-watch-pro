#!/usr/bin/env bash
# Build the .deb from release binaries. Run from anywhere:
#   ./packaging/deb/build.sh [version]
# Version defaults to the latest git tag with the leading 'v' removed.
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

for tool in dpkg-deb install cut sed; do
  command -v "$tool" >/dev/null 2>&1 || { echo "required tool missing: $tool" >&2; exit 2; }
done

DAEMON="$ROOT/target/release/netwatchd"
APP="$ROOT/target/release/netwatch-app"
for bin in "$DAEMON" "$APP"; do
  [[ -x "$bin" ]] || { echo "missing release binary: $bin (run cargo build --release -p netwatchd -p netwatch-app)" >&2; exit 2; }
done

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
OUT_DIR="$ROOT/target/debian"
mkdir -p "$OUT_DIR"

install -d "$STAGE/DEBIAN" \
  "$STAGE/usr/bin" \
  "$STAGE/usr/libexec/netwatch-pro" \
  "$STAGE/usr/share/polkit-1/actions" \
  "$STAGE/usr/share/applications" \
  "$STAGE/usr/share/icons/hicolor/256x256/apps" \
  "$STAGE/lib/systemd/system"

sed "s/@VERSION@/${VERSION}/" "$HERE/control" > "$STAGE/DEBIAN/control"
install -m 0755 "$HERE/postinst" "$HERE/prerm" "$HERE/postrm" "$STAGE/DEBIAN/"
install -m 0755 "$DAEMON" "$STAGE/usr/bin/netwatchd"
install -m 0755 "$APP" "$STAGE/usr/bin/netwatch-app"
install -m 0755 "$ROOT/packaging/polkit/enable-monitoring" \
  "$STAGE/usr/libexec/netwatch-pro/enable-monitoring"
install -m 0644 "$ROOT/packaging/polkit/pro.netwatch.enable-monitoring.policy" \
  "$STAGE/usr/share/polkit-1/actions/pro.netwatch.enable-monitoring.policy"
install -m 0644 "$ROOT/packaging/systemd/netwatchd.service" "$STAGE/lib/systemd/system/netwatchd.service"
install -m 0644 "$HERE/net-watch-pro.desktop" "$STAGE/usr/share/applications/net-watch-pro.desktop"
install -m 0644 "$ROOT/app/src-tauri/icons/icon@2x.png" \
  "$STAGE/usr/share/icons/hicolor/256x256/apps/net-watch-pro.png"

OUT="$OUT_DIR/net-watch-pro_${VERSION}_amd64.deb"
if command -v fakeroot >/dev/null 2>&1; then
  fakeroot dpkg-deb --build "$STAGE" "$OUT"
else
  dpkg-deb --build "$STAGE" "$OUT"
fi
echo "built: $OUT"
