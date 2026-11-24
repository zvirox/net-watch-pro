#!/bin/sh
set -e

if command -v systemctl >/dev/null 2>&1; then
  systemctl stop netwatchd.service 2>/dev/null || true
  systemctl disable netwatchd.service 2>/dev/null || true
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
