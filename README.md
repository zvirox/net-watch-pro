<p align="center">
  <img src="docs/assets/net-watch-pro-logo.png" alt="Net Watch Pro logo" width="220" />
</p>

<h1 align="center">Net Watch Pro</h1>

<p align="center">
  <strong>Local-first Linux network monitoring and per-app data usage. No cloud, no telemetry, no outbound connections.</strong>
</p>

<p align="center">
  <a href="https://github.com/zvirox/net-watch-pro/releases"><img src="https://img.shields.io/github/v/release/zvirox/net-watch-pro?include_prereleases&amp;label=latest%20release" alt="Latest release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/zvirox/net-watch-pro" alt="License" /></a>
  <a href="https://github.com/zvirox/net-watch-pro/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/zvirox/net-watch-pro/ci.yml?label=CI" alt="CI status" /></a>
</p>

<p align="center">
  <a href="https://github.com/zvirox/net-watch-pro/releases">Releases</a> ·
  <a href="PLAN.md">Project plan</a> ·
  <a href="CHECKLIST.md">Live progress checklist</a> ·
  <a href="CHANGELOG.md">Changelog</a> ·
  <a href="#install">Install</a> ·
  <a href="#faq">FAQ</a> ·
  <a href="#troubleshooting">Troubleshooting</a>
</p>

## Project status

Net Watch Pro is a **public beta**. The latest tag is `v0.9.1-beta.3`
(2026-11-21); see the [changelog](CHANGELOG.md) for every release.

Monitoring works end to end:

- A root systemd daemon (`netwatchd`) samples kernel interface counters and,
  where the kernel permits it, attaches an **Aya eBPF** collector for per-socket
  byte accounting and per-app attribution. If eBPF is unavailable the daemon
  stays up in an honest **degraded** state — PC-level speeds still work, and the
  UI says exactly which per-app data is missing.
- An unprivileged Tauri 2 + Svelte 5 UI talks **only** to a localhost Unix
  socket with peer-credential UID authorization. There is no bearer token, no
  cloud, no analytics, and no outbound connection by contract.
- SQLite stores connections, rollups, settings, and diagnostics locally.
  Free tier: live monitoring, per-app usage, 90-day history. Pro: an offline
  license key unlocks advanced filters and CSV/JSON export.

Phases 0–8 (foundation, eBPF spike, doc fixes, walking skeleton, live, history,
dashboard, apps, freemium, system, and installable packaging) are complete.
**Phase 9 — Diagnostics & Accessibility** shipped with the `v0.9.0-beta.1` public beta. **Phase L — Live
Throughput** landed all four slices: live PC speed (A), the eBPF spike (B),
the per-app collector (C), and remembered range presets with an Apps list (D).
Per-task evidence lives in the
[progress checklist](CHECKLIST.md) and the [live-throughput plan](docs/LIVE-THROUGHPUT-PLAN.md).

Firewall features are **out of scope until after v1.0** (PRD §12). This is a
monitor-first product promise, not a roadmap shortcut.

## What works in the current checkout

- **Live PC speed** at 1 Hz from interface counters, with a 60-second sparkline
  and a selectable Mbps / MB/s unit.
- **Per-app live speed** from eBPF cgroup/socket attribution when a collector
  is attached, with low-confidence and unattributed rows shown honestly.
- **Live connections**, usage history, dashboards, per-app detail, budgets,
  retention, settings, offline Pro licensing, and export.
- **Signed packages**: `.deb`, `.rpm`, and an AUR `-git` recipe, built and GPG-
  signed in CI on release tags.
- **Design system**: DESIGN.md tokens implemented as CSS custom properties in
  `app/src/lib/theme/tokens.css`, with self-hosted Inter Variable and JetBrains
  Mono — no font CDN fetch. Light/dark themes, reduced-motion support, and
  keyboard navigation follow DESIGN.md §9.
- **Gates**: `cargo fmt --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, and in `app/`: `npm run check`,
  `npm run lint`, `npm run build`. CI runs these on every push and PR.

## Architecture

~~~mermaid
flowchart LR
  UI["Svelte 5 UI (unprivileged)"]
  API["netwatch-api - Unix socket, peer credentials"]
  Core["netwatch-core - accounting, attribution, retention"]
  EBPF["Aya eBPF - observe only (optional)"]
  DB[("SQLite - connections, rollups, events")]

  UI -->|WS and REST over Unix socket| API
  API --> Core
  EBPF --> Core
  Core --> DB
  DB --> API
~~~

Two processes, one boundary: the root daemon observes traffic and writes local
SQLite; the unprivileged UI talks **only** to the peer-credential-authorized
local API. Details in [ARCHITECTURE.md](ARCHITECTURE.md) and
[AUTH_SECURITY.md](AUTH_SECURITY.md).

## Install

### From GitHub Releases (recommended)

Download the latest signed `.deb` or `.rpm` from the
[releases page](https://github.com/zvirox/net-watch-pro/releases). Each release
publishes `SHA256SUMS` and detached GPG signatures for every artifact.

~~~bash
# Debian / Ubuntu
sudo apt install ./net-watch-pro-*.deb

# Fedora / RHEL
sudo dnf install ./net-watch-pro-*.rpm
~~~

Package installation creates the `netwatch` group, state directory, and socket
GID drop-in, and leaves the daemon unit **disabled**. The first-run onboarding
in the app asks for one root authorization (via `pkexec`) to register your UID
in `/etc/netwatch-pro/allowed-uids` and enable the unit. No background
privilege escalation happens without that prompt.

### Arch (AUR)

The `-git` package builds from source:

~~~bash
paru -S net-watch-pro-git
# or: yay -S net-watch-pro-git
~~~

### From source

See [Development setup](#development-setup), then build release binaries with
`cargo build --release` and the UI with `cd app && npm run build`. Packaged
builds embed the eBPF object in `netwatchd`, so there is no object file to
install and no manual step for users. Building from source yourself still
needs the development BPF prerequisites described below.

### Uninstall

Package removal deletes local data by default. To keep it, create
`/etc/netwatch-pro/keep-data` before removing the package. Removal also drops
the unit override, the allowed-UID list, and the config directory.

## Development setup

### Requirements

- Linux, Rust **1.99.1** (pinned in `rust-toolchain.toml`), Node.js 20+.
- Tauri 2 system dependencies: WebKit2GTK 4.1, GTK 4, `libayatana-appindicator`,
  and the usual build tools. See your distro's Tauri v2 guide; a desktop
  session with D-Bus is required for the UI.
- Root access is **not** needed for the UI gate. eBPF attach and the pkexec
  onboarding flow need root and are checked manually per
  [TESTING.md](TESTING.md).

### Build and verify

~~~bash
git clone https://github.com/zvirox/net-watch-pro.git
cd net-watch-pro

# Rust gate
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# UI gate
cd app
npm install
npm run check
npm run lint
npm run build
~~~

For a live dev UI, start `npm run dev` in `app/` before launching the debug
Tauri binary — debug builds load the Vite dev URL from `tauri.conf.json`.

#### Building the eBPF object for development

Packaged builds do this automatically. To build a daemon with an embedded
collector yourself, install `rust-src` and `bpf-linker` (both available in the
Arch `extra` repo; CI pins `bpf-linker` 0.13.0), then:

~~~bash
cargo run -p xtask -- bpf                 # writes target/netwatch_ebpf.o
cargo build --release -p netwatchd   # build.rs embeds that object
~~~

`cargo run -p xtask -- bpf` uses stable Rust with `RUSTC_BOOTSTRAP=1`, `rust-src` and
`bpf-linker`; no nightly toolchain is required. To iterate on the BPF program
without rebuilding the daemon, set `NETWATCH_EBPF_OBJECT` to the object path.
The object is never committed to git.

### Documentation-first rule

The specs are the contract, in priority order: `PRD.md` → `DESIGN.md` →
`ARCHITECTURE.md` → `DATABASE.md` → `API.md`. Code that drifts from them is
wrong. [AGENTS.md](AGENTS.md) is the binding rulebook for human and AI
contributors. Configuration variables are listed in
[docs/env-vars.md](docs/env-vars.md).

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `NETWATCH_DATA_DIR` | `/var/lib/netwatch-pro` | Database and state directory |
| `NETWATCH_API_SOCKET` | `/run/netwatch-pro/api.sock` | Unix socket path |
| `NETWATCH_API_GID` | `SUDO_GID` or root GID | Group allowed to connect (socket is `0660`) |
| `NETWATCH_RETENTION_DAYS` | `90` | Default retention (Pro can exceed) |
| `NETWATCH_DISABLE_EBPF` | unset | Run with no collector (degraded state) |
| `NETWATCH_EBPF_OBJECT` | unset | Dev override to load the object from a file instead of the embedded bytes |
| `NETWATCH_LOG_LEVEL` | `info` | `error` / `warn` / `info` / `debug` / `trace` |

The full closed list is in [docs/env-vars.md](docs/env-vars.md).

## FAQ

**Does Net Watch Pro send my traffic anywhere?** No. There is no cloud,
telemetry, analytics, update ping, or CDN font fetch. Data stays in local
SQLite. The UI's own Live view is designed to make the zero-outbound promise
checkable.

**Why does the status chip say "Degraded"?** The daemon is monitoring, but a
per-app eBPF collector is not feeding data. PC-level speeds still work. Common
causes: missing eBPF object, a kernel that denies BPF, or `NETWATCH_DISABLE_EBPF`
being set.

**What is "Unattributed" traffic?** Interface-counter totals minus bytes the
collector could attribute to apps — headers, ACKs, retransmits, and kernel/VPN
traffic. It is shown explicitly rather than silently assigned to an app.

**Is my history kept forever?** The free tier keeps 90 days. Pro extends
retention with an offline license key; no account or online activation is
required.

**Can it block traffic?** Not yet. Firewall features are planned only after
v1.0 (PRD §12). Net Watch Pro is monitor-only today.

**Why do I need root once?** Only to register your UID and enable the daemon at
onboarding. Day-to-day UI use is unprivileged and talks to the daemon over a
peer-credential-checked Unix socket.

## Troubleshooting

**UI shows "Connection refused" or "Connecting…" in a debug build.** Debug
Tauri binaries load the Vite dev URL. Start `npm run dev` in `app/`, or use a
release package. If the daemon is linked, its `/status` reports
`ws_clients: 1`.

**"Authorization needed" appears.** Your UID is not in
`/etc/netwatch-pro/allowed-uids`. Re-run onboarding, or have root add your
numeric UID to that file. Socket access also requires membership of the group
set by `NETWATCH_API_GID` (normally `netwatch`).

**Chip is Degraded / per-app speeds missing.** Packaged builds carry the
collector object already, so check the daemon log and Diagnostics modal for
the attach failure. A locked-down kernel or missing `CAP_BPF`/`CAP_PERFMON`
can prevent attach — the daemon degrades rather than crashing. A developer
override via `NETWATCH_EBPF_OBJECT` is supported but not needed for installs.

**Dashboard says "Stale" or stays empty.** Confirm the unit is enabled and
running (`systemctl status netwatchd`) and that the daemon can see host
interfaces under `/sys/class/net`. A network-namespace mismatch has caused
empty dashboards before; the hardened unit is configured to avoid it.

**Missing shared libraries when launching the UI.** Install the full Tauri 2
system dependency set for your distribution (WebKit2GTK 4.1, GTK 4, app
indicator). The packaged builds bundle the frontend but still need these
system libraries.

**Need to report a bug?** Include the release version, distribution, desktop
session (X11/Wayland), and reproduction steps in the
[issue tracker](https://github.com/zvirox/net-watch-pro/issues). For security
issues, follow [SECURITY.md](SECURITY.md) — never open a public issue.

## Releases and feedback

Release gates, tagging, and the alpha→beta→1.0 cadence are documented in
[PLAN.md](PLAN.md); tagged changes live in the [changelog](CHANGELOG.md).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a change. In short:
plan first (name the governing doc sections), keep diffs small and scoped to
one task, ship tests with the code ([TESTING.md](TESTING.md) §8 gate), sync the
affected docs in the same change, and use Conventional Commits. New
dependencies require maintainer approval first.

| Area | Examples | Difficulty |
|---|---|---|
| Testing and bug reports | Run the gates on your distro and report what breaks | Easy |
| Documentation | Clarify setup, specs, and workflows | Easy |
| UI polish | Missing states, a11y fixes, keyboard shortcuts | Easy to medium |
| Test coverage | Accounting/attribution accuracy tests (`tests/accuracy/`) | Medium |
| Packaging | Arch, deb, rpm pipelines and verification | Medium |
| eBPF / kernel | Aya attach targets, kernel-version quirks | Hard |

## License

Net Watch Pro is **dual-licensed**:

- **AGPL-3.0-or-later** — free to use, study, modify, and share under the GNU
  Affero General Public License v3 or later ([LICENSE](LICENSE)).
- **Commercial license** — for contributors and users who cannot comply with
  AGPL terms (e.g. embedding Net Watch Pro in proprietary products or
  distributions). Contact the maintainer for terms.

By contributing you agree that your work is released under both options.
