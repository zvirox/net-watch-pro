#!/bin/sh
set -e

if ! getent group netwatch >/dev/null 2>&1; then
  groupadd --system netwatch || true
fi

GID="$(getent group netwatch 2>/dev/null | cut -d: -f3 || echo 0)"

install -d -m 0755 /etc/netwatch-pro
install -d -m 0750 /var/lib/netwatch-pro 2>/dev/null || true

install -d -m 0755 /etc/systemd/system/netwatchd.service.d
cat > /etc/systemd/system/netwatchd.service.d/10-socket-group.conf <<EOF
[Service]
Environment=NETWATCH_API_GID=${GID}
EOF

if command -v systemctl >/dev/null 2>&1; then
  systemctl daemon-reload 2>/dev/null || true
  if systemctl is-active --quiet netwatchd.service 2>/dev/null; then
    systemctl restart netwatchd.service >/dev/null 2>&1 || true
  fi
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
