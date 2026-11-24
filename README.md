<p align="center">
  <img src="docs/assets/net-watch-pro-logo.png" alt="Net Watch Pro logo" width="180" />
  <h1 align="center">Net Watch Pro</h1>
</p>

<p align="center">
  <strong>Private, local-first Linux network monitoring and per-application data usage</strong>
</p>

<p align="center">
  <a href="https://github.com/zvirox/net-watch-pro/releases"><img src="https://img.shields.io/badge/Release-Latest_Packages-blue.svg" alt="Releases" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License" /></a>
  <img src="https://img.shields.io/badge/Platform-Linux_(Wayland_&_X11)-orange.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/Stack-Rust_·_Tauri_2_·_React_19-indigo.svg" alt="Stack" />
</p>

<p align="center">
  <a href="#what-is-net-watch-pro">What is it?</a> ·
  <a href="#key-features">Key Features</a> ·
  <a href="#starship-control-integration">Starship Control Integration</a> ·
  <a href="#installation">Installation</a> ·
  <a href="#frequently-asked-questions">FAQ</a>
</p>

---

## What is Net Watch Pro?

Net Watch Pro is a lightweight, privacy-focused network monitor and per-app data tracker built exclusively for Linux. Most network tools either provide basic global bandwidth graphs without showing which app is responsible, or require sending your telemetry to cloud dashboards.

Net Watch Pro runs entirely on your own machine. It pairs a root background daemon that safely inspects traffic via Linux kernel counters and eBPF with an unprivileged desktop interface. You can see your live network speeds, inspect active connections, and track how much data each application consumes over time without a single byte leaving your computer.

---

## Key Features

- **Live Speed Monitoring**: Track real-time download and upload speeds at 1 Hz with smooth 60-second sparklines and selectable Mbps or MB/s display units.
- **Per-Application Attribution**: Identify exactly which program is downloading or uploading data using high-speed eBPF socket inspection.
- **Historical Data Usage**: Review detailed usage records by hour, day, week, or month. Keep up to 90 days of history locally on disk in a lightweight SQLite database.
- **Active Connection Inspector**: View live outbound connections, remote IP addresses, ports, and protocols for any running application.
- **Offline License Support**: Advanced filtering options and CSV or JSON data export are unlocked with an offline license key. No online validation servers are ever contacted.
- **Zero Cloud by Design**: No telemetry, no background analytics, and no remote connection requirements.
- **Modern Linear-Style Design**: Built with Tauri 2, React 19, and Tailwind CSS v4, featuring a distraction-free dark interface with high-contrast charts.

---

## Starship Control Integration

Net Watch Pro is a fully standalone application. You can view speeds, inspect connection graphs, and review bandwidth history without installing any companion tools.

To enable automated system monitoring, Net Watch Pro links seamlessly with [Starship Control](https://github.com/zvirox/starship-control) ([Releases](https://github.com/zvirox/starship-control/releases)), the unified Linux desktop AI assistant.

### Why It Exists

When an application starts downloading a massive file or a background process slows down your connection, finding the culprit manually can be frustrating. Connecting Net Watch Pro to Starship Control lets you query your network health using conversational prompts and receive automatic proactive alerts.

### Real-World Use Cases

- **Immediate Bandwidth Diagnosis**: Ask Starship Control *"Which application is using the most network bandwidth right now?"* The assistant identifies the top data consumer instantly.
- **Daily Usage Summaries**: Request a quick overview: *"How much network data did my web browser download today?"*
- **Proactive Anomaly Alerts**: Have Starship Control alert you whenever an unexpected background process starts transmitting heavy data while you are working.

### How Connection Works

Setup requires no manual configuration:

1. **Automatic Discovery**: When Net Watch Pro starts, it registers its tool definition in `$XDG_DATA_HOME/starship/plugins/nwp.json`.
2. **Private Local Socket**: Communication travels across a private Unix domain socket at `$XDG_RUNTIME_DIR/starship/bridges/nwp.sock`.
3. **Linux Kernel Peer Verification**: The connection checks Linux peer credentials (`SO_PEERCRED`) on every message. Only applications running under your exact user ID can view data. No network ports are opened, and your traffic logs remain strictly confidential.
4. **Instant Assistant Access**: Summon Starship Control with `Super+Shift+Space`. It immediately detects Net Watch Pro and provides live network insights upon request. For setup guides and downloads, visit the [Starship Control repository](https://github.com/zvirox/starship-control) and [releases](https://github.com/zvirox/starship-control/releases).

---

## Installation

Prebuilt Linux packages are available on the [Releases page](https://github.com/zvirox/net-watch-pro/releases).

### Debian, Ubuntu, and Pop!_OS (`.deb`)

```bash
sudo apt install ./net-watch-pro_*_amd64.deb
```

### Fedora, RHEL, and openSUSE (`.rpm`)

```bash
sudo dnf install ./net-watch-pro-*.x86_64.rpm
```

### Arch Linux and Manjaro (`.pkg.tar.zst`)

```bash
sudo pacman -U ./net-watch-pro-*-x86_64.pkg.tar.zst
```

### Building From Source

Requirements: Rust stable, Node.js 22+, and Linux development packages (`libglib2.0-dev`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `librsvg2-dev`).

```bash
git clone https://github.com/zvirox/net-watch-pro.git
cd net-watch-pro
cd app && npm install && npm run build && cd ..
cargo build --release -p netwatch-app -p netwatchd
```

---

## Frequently Asked Questions

#### Does Net Watch Pro block network traffic like a firewall?
No. Net Watch Pro is strictly a monitoring and accounting tool. It observes network packet counts and socket states to help you understand your data usage without interfering with your connections.

#### Does Net Watch Pro send any network data to the cloud?
No. Net Watch Pro has a strict zero-telemetry architecture. All connection logs and usage totals are stored locally in an unencrypted SQLite file on your disk.

#### Is Starship Control required to use Net Watch Pro?
No. Net Watch Pro is a complete standalone program with its own visual interface and tray menu. Starship Control is an optional companion for voice and automated agentic workflows.

#### How does per-app attribution work?
On modern Linux kernels, the background daemon uses eBPF to trace socket ownership back to the process ID and executable name. If eBPF is unavailable on your system, Net Watch Pro gracefully falls back to system-wide network counters.

---

## License

Net Watch Pro is open-source software licensed under the [Apache License, Version 2.0](LICENSE).
