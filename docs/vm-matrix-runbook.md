# VM matrix runbook — Phase 10 P10.4

**Purpose:** produce the evidence required by PRD §11 DoD #1 and PLAN P10.4:
Net Watch Pro installs, onboards, monitors, degrades and uninstalls correctly on
**real virtual machines** of all three target families. This is the gate that
CI **container** smoke cannot satisfy (container runs share the host kernel and
cannot validate BPF/kernel variance).

**Status:** runbook prepared; runs are **owner-executed**. No row may be marked
`[VERIFIED]` until its outputs are pasted into `CHECKLIST.md` with the date.

## Scope and environment

| Item | Requirement |
|---|---|
| Machines | Three **fresh VMs**: Arch/CachyOS, Ubuntu 24.04 LTS, Fedora 41 |
| Size | ≥ 2 vCPU, 4 GB RAM, 30 GB disk; nested virtualisation off is fine (monitor-only) |
| Session | Graphical desktop with D-Bus and a polkit agent (for the one pkexec onboarding prompt) |
| User | One sudo-capable desktop user (the test user); no root desktop session |
| Kernel | cgroup v2, BTF present, `CAP_BPF`/`CAP_PERFMON` available to root |
| Artifacts | Signed release assets from <https://github.com/zvirox/net-watch-pro/releases/tag/v0.9.2-beta.4> (or the newest published beta) |
| Network | Needed only to download packages and generate test traffic; app itself must make **no** outbound connection |

Take a VM snapshot **before installation** so the uninstall tests can be
repeated from a clean state.

### Preflight (run on each VM)

```bash
uname -a
sudo test -f /sys/kernel/btf/vmlinux && echo "BTF: present" || echo "BTF: MISSING"
sudo test -f /sys/fs/cgroup/cgroup.controllers && head -1 /sys/fs/cgroup/cgroup.controllers
id -u; groups
loginctl show-session "$XDG_SESSION_ID" -p Type    # must be a graphical session
```

If BTF or cgroup v2 is missing, record the VM as **blocked** (not failed) and
attach the output; eBPF attach cannot be validated on that kernel.

## 1. Download and verify artifacts

On a machine with `curl` and `gpg`, download the release files for the target
family plus `SHA256SUMS`, `SHA256SUMS.asc` and `nwp-release-public.asc`:

```bash
gpg --import nwp-release-public.asc
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
gpg --verify net-watch-pro_0.9.2-beta.4_amd64.deb.asc \
           net-watch-pro_0.9.2-beta.4_amd64.deb
```

Use the matching `.asc` for the `.rpm` or source tarball. Record the verification
output in the evidence table.

## 2. Install per distro

### Ubuntu 24.04 LTS (deb)

```bash
sudo apt update
sudo apt install ./net-watch-pro_0.9.2-beta.4_amd64.deb
```

### Fedora 41 (rpm)

```bash
sudo dnf install ./net-watch-pro-0.9.2.beta.4-1.x86_64.rpm
```

### Arch / CachyOS (AUR `-git` dev channel)

The AUR package is not published; use the release's AUR source bundle. It
builds current `main` and needs build dependencies:

```bash
tar -xzf net-watch-pro-git-src.tar.gz
cd net-watch-pro-git-src      # contains PKGBUILD, .SRCINFO, install hook
makepkg -si
```

Record the commit that was built (`git describe --always` in the build output;
`-git` tracks `main`, not the release tag).

### Post-install checks (all distros)

```bash
systemctl status netwatchd.service           # present, inactive, disabled
getent group netwatch                        # socket group exists
grep NETWATCH_API_GID /etc/systemd/system/netwatchd.service.d/10-socket-group.conf
ls -l /usr/share/applications/net-watch-pro.desktop
systemd-analyze security netwatchd.service   # compare with P10.5 review baseline
```

The unit is intentionally **disabled**; onboarding must enable it.

## 3. Onboarding (time and prompt count)

```bash
START=$(date +%s)
netwatch-app &          # or launch from the desktop menu
# Complete the onboarding cards; count visible pkexec prompts (must be exactly 1)
END=$(date +%s)
echo "onboarding_seconds=$((END-START))"       # must be < 180
journalctl -b --since "@$START" --until "@$END" | grep -Ei 'pkexec|polkit' || true
```

After onboarding:

```bash
systemctl is-active netwatchd.service          # active
cat /etc/netwatch-pro/allowed-uids             # contains the desktop user's UID
ls -l /run/netwatch-pro/api.sock               # srw-rw---- root:netwatch
```

Expected: one prompt, health checks pass, the Dashboard lands with live data,
and no further prompt appears on restart of the app.

## 4. Functional and honesty checks

Generate real traffic: browse a site, run a download for ~30 s, toggle a VPN if
available. Leave the app open while running these checks.

```bash
S=/run/netwatch-pro/api.sock
curl -s --unix-socket "$S" http://localhost/api/v1/health
curl -s --unix-socket "$S" http://localhost/api/v1/status
curl -s --unix-socket "$S" 'http://localhost/api/v1/usage/summary?range=24h'
curl -s --unix-socket "$S" http://localhost/api/v1/connections/live
curl -s --unix-socket "$S" 'http://localhost/api/v1/connections?limit=5'
curl -s --unix-socket "$S" http://localhost/api/v1/diagnostics
curl -s --unix-socket "$S" http://localhost/api/v1/license
```

Pass conditions:

| Check | Expected |
|---|---|
| `status` | `daemon=monitoring`, `ebpf=attached`, `recording=true`, real `bpf_packets_seen` > 0 |
| eBPF maps | `sudo bpftool prog list \| grep cgroup_skb` shows ingress+egress; `bpftool map list` shows `CONFIG`, `BYTES`, `STATS`, `FLOWS` |
| Dashboard | live PC speed non-zero during traffic; Unattributed ≥ 0 and formatted (no raw byte string); totals plausible vs `vnstat` |
| Live | at least one row with app name, protocol, remote host/port and non-zero rate; unknown PID renders `—` |
| History | rows appear after traffic closes; pagination loads further pages without jumping to page 1 |
| Apps / App detail | live speed columns and range totals populated for top apps |
| Diagnostics | checks show `pass`/`fail`/`skip` with detail; journal excerpt populated; Copy report works |
| Trust | `sudo ss -tunp \| grep netwatch` shows only Unix sockets; no AF_INET connections from the app |

Interface-counter cross-check (DoD #2, with traffic flowing):

```bash
IFACE=$(ip route get 1.1.1.1 | awk '/dev/{print $5; exit}')
BEFORE=$(cat /sys/class/net/$IFACE/statistics/rx_bytes)
# ... 60 s of download ...
AFTER=$(cat /sys/class/net/$IFACE/statistics/rx_bytes)
echo "kernel_delta=$((AFTER-BEFORE))"
curl -s --unix-socket "$S" 'http://localhost/api/v1/usage/summary?range=1h'
```

Record both numbers; CI's seeded accuracy harness covers the ≥95% attribution
gate, this check confirms the VM's headline total tracks the kernel counter.

## 5. Degraded states (PRD §11 #5)

```bash
# No daemon
sudo systemctl stop netwatchd.service
# UI: chip Stopped, non-dismissible banner, no fresh-looking numbers
sudo systemctl start netwatchd.service
# UI recovers to Monitoring within 5 s
```

```bash
# No eBPF collector (release build honours this diagnostic switch)
sudo systemctl stop netwatchd.service
sudo env NETWATCH_DISABLE_EBPF=1 /usr/bin/netwatchd >/tmp/nwp-noebpf.log 2>&1 &
sleep 2
curl -s --unix-socket "$S" http://localhost/api/v1/status   # ebpf=no_collector
# UI chip Degraded; Dashboard/Live/Apps explain missing per-app data
sudo pkill -f '^/usr/bin/netwatchd$'
sudo systemctl start netwatchd.service
```

```bash
# Permission denied: a UID not in the allowlist must be refused
sudo useradd -m nwp-denied || true
sudo -u nwp-denied curl -s --unix-socket "$S" http://localhost/api/v1/health
# expected: connection refused / permission denied
sudo userdel -r nwp-denied
```

## 6. Pro/free gating (PRD §11 #6)

The debug `xtask license-dev-key` path is **rejected by release binaries**. Use
a real signed key supplied by the owner from release infrastructure.

1. Record free state: `license.tier=free`, export and advanced filters locked,
   retention ceiling 90 days.
2. Open Upgrade/License, paste the production key → `tier=pro`, export and
   advanced filters unlock immediately, ceiling 365 days, no restart.
3. Export CSV and JSON via the native dialog; open the files and check contents.
4. Apply an advanced History filter; confirm results.
5. Deactivate → all gates return to free.

If no production key is available on a VM, record this row as **blocked** with
that reason — do not substitute a dev key and do not mark it verified.

## 7. Uninstall (PRD §11 uninstall / P8.7)

Test the opt-in marker first, then restore the snapshot and test default
removal:

```bash
sudo touch /etc/netwatch-pro/keep-data
sudo apt remove net-watch-pro          # or dnf remove / pacman -R
test -d /var/lib/netwatch-pro && echo "data kept (expected)"
# restore VM snapshot, install again, then:
sudo apt remove net-watch-pro
test ! -d /var/lib/netwatch-pro && echo "data removed (expected)"
systemctl status netwatchd.service    # no unit residue
test ! -f /etc/netwatch-pro/allowed-uids
```

## 8. Evidence template (copy per VM)

```text
Distro/version:
Kernel / BTF / cgroup2:
Release tag + artifact checksums verified:
Install command:
Install result:
Onboarding seconds / pkexec prompt count:
status JSON:
bpftool programs/maps:
Dashboard + Live + History observations:
Unattributed vs kernel counter:
Degraded states (no daemon / no eBPF / permission denied):
Pro activation + export + deactivate:
Uninstall default + keep-data:
Zero-outbound check:
systemd-analyze security score:
Blockers / notes:
```

Paste the completed blocks into `CHECKLIST.md` P10.4 with the run date. Only
after **all three** VMs pass every row, together with P10.6 and the crash-free
metric, may the Phase 10 DoD be signed and `v0.9.2-beta.4` released.
