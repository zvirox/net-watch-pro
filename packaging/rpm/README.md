# RPM packaging (P8.3)

Mirrors the `.deb` recipe for rpm-based distros (QA target: Fedora). The dnf
repo itself is deferred until after beta (PLAN F-10).

## Layout

| File | Purpose |
|---|---|
| `net-watch-pro.spec` | RPM spec: prebuilt binaries, hardened unit, desktop entry, icon |
| `build.sh` | Stages sources into a private rpmbuild tree and runs `rpmbuild -bb` |

The desktop entry is reused from `packaging/deb/`; the systemd unit comes from
`packaging/systemd/netwatchd.service`.

## Build

```bash
cd <repo>
cargo build --release -p netwatchd -p netwatch-app
./packaging/rpm/build.sh 0.8.0~alpha.8
# output: target/rpm/net-watch-pro-0.8.0.alpha.8-1.x86_64.rpm
```

With no version argument, the latest git tag (minus `v`) is used. `~` is
rewritten to `.` because RPM and Debian spell pre-release versions differently.

## Install / verify

```bash
sudo dnf install ./target/rpm/net-watch-pro-*.rpm
systemctl status netwatchd.service        # present, inactive, disabled
getent group netwatch                     # socket group exists
rpm -qpl net-watch-pro | grep netwatchd.service
rpm -ql net-watch-pro | grep 10-socket-group.conf
```

Onboarding (P8.5) adds the desktop user to `netwatch` and enables the unit after
health checks — the package deliberately does neither.

`dnf remove` removes the unit drop-in and allowed-UID list and **deletes
`/var/lib/netwatch-pro` by default**. To keep recorded data, create
`/etc/netwatch-pro/keep-data` before uninstalling (PRD opt-in retention, P8.7).
