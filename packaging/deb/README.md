# Debian packaging (P8.2)

Produces a source-controlled `.deb` for deb-based distros (QA target: Ubuntu
LTS). The apt repo itself is deferred until after beta (PLAN F-10).

## Layout

| File | Purpose |
|---|---|
| `control` | Package metadata; `@VERSION@` is substituted at build time |
| `postinst` | Creates the `netwatch` socket group, state dir and systemd drop-in; **leaves the unit disabled** |
| `prerm` / `postrm` | Stop/disable the service, remove the drop-in; `purge` removes `/var/lib/netwatch-pro` |
| `net-watch-pro.desktop` | Desktop menu entry |
| `build.sh` | Stages release binaries + the hardened unit and calls `dpkg-deb` |

## Build

Requires `dpkg-deb` (and optionally `fakeroot`) plus release binaries:

```bash
cd <repo>
cargo build --release -p netwatchd -p netwatch-app
./packaging/deb/build.sh 0.8.0~alpha.8
# output: target/debian/net-watch-pro_0.8.0~alpha.8_amd64.deb
```

With no version argument, the latest git tag (minus `v`) is used.

## Install / verify

```bash
sudo apt install ./target/debian/net-watch-pro_*.deb
systemctl status netwatchd.service        # present, inactive, disabled
getent group netwatch                     # socket group exists
ls -l /usr/share/applications/net-watch-pro.desktop
grep NETWATCH_API_GID /etc/systemd/system/netwatchd.service.d/10-socket-group.conf
```

Onboarding (P8.5) adds the desktop user to `netwatch` and enables the unit
after health checks — the package deliberately does neither.

## Uninstall

`apt remove` stops the service, removes the unit drop-in and allowed-UID list,
and **deletes `/var/lib/netwatch-pro` by default**. To keep recorded data,
create `/etc/netwatch-pro/keep-data` before uninstalling. `apt purge` always
deletes the data, regardless of the marker (PRD §6.1 #10 / P8.7).
