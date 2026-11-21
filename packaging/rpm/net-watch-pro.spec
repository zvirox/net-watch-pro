# Net Watch Pro RPM package (P8.3). Built from prebuilt release binaries via
# packaging/rpm/build.sh; the dnf repo itself is deferred until after beta.

%define nwp_version 0.8.0.alpha.8
%{!?_unitdir:%define _unitdir %{_prefix}/lib/systemd/system}
%define nwp_datadir /var/lib/netwatch-pro

Name:           net-watch-pro
Version:        %{nwp_version}
Release:        1%{?dist}
Summary:        Local network-traffic monitor and per-app usage tracker

License:        AGPL-3.0-or-later
URL:            https://github.com/zvirox/net-watch-pro
BuildArch:      x86_64

Requires:       systemd
Requires:       webkit2gtk4.1
Requires:       librsvg2
# The tray's appindicator library is optional at runtime (degraded if absent)
# and its package name varies across Fedora releases, so it is not hard-required.

Source0:        netwatchd
Source1:        netwatch-app
Source2:        netwatchd.service
Source3:        net-watch-pro.desktop
Source4:        net-watch-pro.png
Source5:        LICENSE
Source6:        enable-monitoring
Source7:        pro.netwatch.enable-monitoring.policy

%description
Net Watch Pro observes network traffic on this device and records per-application
data usage with searchable local history. A root daemon serves a peer-credential
Unix socket API to an unprivileged Tauri desktop UI.

Free tier: live view, per-app usage, 90-day history, budgets and alerts. Pro
(offline license key): advanced filters, CSV/JSON export and 365-day retention.

No telemetry, no analytics, no update pings, no outbound connections.

%prep
# Prebuilt binaries are staged by build.sh; nothing to unpack.

%build
# Nothing to compile here.

%install
install -d %{buildroot}%{_bindir}
install -d %{buildroot}%{_libexecdir}/netwatch-pro
install -d %{buildroot}%{_datadir}/polkit-1/actions
install -d %{buildroot}%{_unitdir}
install -d %{buildroot}%{_sysconfdir}/systemd/system/netwatchd.service.d
install -d %{buildroot}%{_datadir}/applications
install -d %{buildroot}%{_datadir}/icons/hicolor/256x256/apps
install -d %{buildroot}%{_datadir}/licenses/net-watch-pro
install -d %{buildroot}%{nwp_datadir}

install -m 0755 %{SOURCE0} %{buildroot}%{_bindir}/netwatchd
install -m 0755 %{SOURCE1} %{buildroot}%{_bindir}/netwatch-app
install -m 0755 %{SOURCE6} %{buildroot}%{_libexecdir}/netwatch-pro/enable-monitoring
install -m 0644 %{SOURCE7} %{buildroot}%{_datadir}/polkit-1/actions/pro.netwatch.enable-monitoring.policy
install -m 0644 %{SOURCE2} %{buildroot}%{_unitdir}/netwatchd.service
install -m 0644 %{SOURCE3} %{buildroot}%{_datadir}/applications/net-watch-pro.desktop
install -m 0644 %{SOURCE4} %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/net-watch-pro.png
install -m 0644 %{SOURCE5} %{buildroot}%{_datadir}/licenses/net-watch-pro/LICENSE

%post
# Group allowed to connect to the root daemon's 0660 API socket.
getent group netwatch >/dev/null 2>&1 || groupadd --system netwatch
GID="$(getent group netwatch | cut -d: -f3)"
install -d -m 0755 %{_sysconfdir}/netwatch-pro
install -d -m 0750 -o root -g netwatch %{nwp_datadir}
install -d -m 0755 %{_sysconfdir}/systemd/system/netwatchd.service.d
printf '[Service]\nEnvironment=NETWATCH_API_GID=%s\n' "$GID" \
  > %{_sysconfdir}/systemd/system/netwatchd.service.d/10-socket-group.conf
# Deliberately NOT enabled: onboarding enables after health checks (P8.5).
systemctl daemon-reload >/dev/null 2>&1 || :
# $1 is 1 on install, 2+ on upgrade. Restart only an already-active unit so
# the new binary serves the API; never enable/start a stopped unit (P8.12).
if [ "$1" -ge 2 ] && systemctl is-active --quiet netwatchd.service >/dev/null 2>&1; then
  systemctl restart netwatchd.service >/dev/null 2>&1 || :
fi
exit 0

%preun
if [ "$1" -eq 0 ]; then
  systemctl stop netwatchd.service >/dev/null 2>&1 || :
fi
exit 0

%postun
if [ "$1" -eq 0 ]; then
  # Defensive removal of the generated drop-in (also declared %ghost).
  rm -f %{_sysconfdir}/systemd/system/netwatchd.service.d/10-socket-group.conf
  rmdir %{_sysconfdir}/systemd/system/netwatchd.service.d 2>/dev/null || :
  # Final uninstall: delete data unless the admin opted in to keeping it.
  keep=0
  if [ -f %{_sysconfdir}/netwatch-pro/keep-data ]; then
    keep=1
  fi
  if [ "$keep" -eq 0 ]; then
    rm -rf %{nwp_datadir}
  fi
  rm -f %{_sysconfdir}/netwatch-pro/allowed-uids %{_sysconfdir}/netwatch-pro/keep-data
  rmdir %{_sysconfdir}/netwatch-pro 2>/dev/null || :
fi
# The ghost drop-in is removed by RPM; refresh systemd either way.
systemctl daemon-reload >/dev/null 2>&1 || :
exit 0

%files
%license %{_datadir}/licenses/net-watch-pro/LICENSE
%dir %{_sysconfdir}/systemd/system/netwatchd.service.d
%ghost %config(noreplace) %{_sysconfdir}/systemd/system/netwatchd.service.d/10-socket-group.conf
%{_bindir}/netwatchd
%{_bindir}/netwatch-app
%dir %{_libexecdir}/netwatch-pro
%{_libexecdir}/netwatch-pro/enable-monitoring
%{_datadir}/polkit-1/actions/pro.netwatch.enable-monitoring.policy
%{_unitdir}/netwatchd.service
%{_datadir}/applications/net-watch-pro.desktop
%{_datadir}/icons/hicolor/256x256/apps/net-watch-pro.png
# post creates the dir owned by root:netwatch; declare root:root here so the
# spec builds even on hosts where the group has not been created yet.
%dir %attr(0750,root,root) %{nwp_datadir}

%changelog
* Thu Nov 19 2026 Net Watch Pro maintainers - 0.8.0.alpha.8-1
- Phase 8 first installable RPM build (source-controlled recipe).
