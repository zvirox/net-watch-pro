# Net Watch Pro

**See exactly which apps use your data and your network — privately, locally, in real time.**

Net Watch Pro is a network traffic monitor and per-app data-usage tracker for Linux. A root daemon observes traffic with eBPF and writes local history; an unprivileged desktop UI shows live connections, per-app usage, and a searchable history. Monitor-first: firewall features arrive after V1.0, step by step.

> **Status: documentation phase.** This repository currently contains the product's complete specification set (7 documents) and **no application code yet**. Everything below marked *(planned)* describes what the specs define, not what exists today.

## Screenshots

*(planned — placeholders until the UI exists)*

| Dashboard | Live Connections | History |
|---|---|---|
| `![Dashboard](docs/img/dashboard.png)` | `![Live](docs/img/live.png)` | `![History](docs/img/history.png)` |
| *usage timeline, top apps, totals* | *real-time per-app connection table* | *searchable 90-day history* |

## Key Features

- **Live Connections** — real-time table of active connections: app, PID, protocol, remote endpoint, rate (PRD §6.1 #1)
- **Per-app usage accounting** — data used per app for today/week/month, with top-apps ranking (§6.1 #2)
- **Dashboard** — usage timeline, top-talker chart, headline totals, per-interface breakdown (§6.1 #3)
- **App detail pages** — per-app drill-down: usage over time, connections, endpoint summary (§6.1 #4)
- **Local history** — 90-day default retention, automatic pruning, everything in SQLite (§6.1 #5)
- **Freemium** — free tier as above; a Pro license key unlocks advanced history filters, CSV/JSON export, and retention up to 1 year (§6.1 #9)
- **Local-only, verifiable** — no cloud, no telemetry, no outbound connections; the app's own Live view can prove it (PRD §4)

## Tech Stack

| Layer | Choice |
|---|---|
| Desktop shell | Tauri 2 |
| Frontend | Svelte 5 + shadcn-svelte |
| Charts | ECharts |
| Daemon backend | Rust + Tokio |
| Traffic monitoring | Aya eBPF |
| Database | SQLite (WAL) |
| Platforms | Arch / CachyOS, deb- and rpm-based distros *(planned: AUR, apt repo, dnf repo)* |

## Quick Start

**Today (documentation phase):**

1. Clone the repository: `git clone <repo-url> && cd net-watch-pro`
2. Read `AGENTS.md` first if you (or your AI assistant) will touch anything — it is the working rulebook.
3. Read the specs in priority order: `PRD.md` → `DESIGN.md` → `ARCHITECTURE.md` → `DATABASE.md` → `API.md`.

**Later, once packages exist** *(planned — per PRD §8 Flow 1)*:

1. Install via AUR: `paru -S net-watch-pro`, or add the apt/dnf repo and install the package.
2. Launch Net Watch Pro from the app menu.
3. Click **Enable monitoring** and confirm the one-time admin (`pkexec`) prompt.
4. The dashboard shows live data within seconds.
## Environment Variables

Full list from ARCHITECTURE.md §8.3 — these are the *only* variables the product defines. *(All daemon-side; none take effect until the daemon is implemented.)*

| Name | Purpose | Example value |
|---|---|---|
| `NETWATCH_DATA_DIR` | DB + state directory (set by systemd) | `/var/lib/netwatch-pro` |
| `NETWATCH_LOG_DIR` | Log output directory | `/var/lib/netwatch-pro/log` |
| `NETWATCH_LOG_LEVEL` | Log verbosity | `info` |
| `NETWATCH_API_ADDR` | Loopback-only API bind address | `127.0.0.1:7897` |
| `NETWATCH_API_TOKEN_FILE` | Where the per-boot API token is written (mode 0600) | `/var/lib/netwatch-pro/api-token` |
| `NETWATCH_RETENTION_DAYS` | Default history retention | `90` |
| `NETWATCH_DISABLE_EBPF` | Diagnostic switch: run without the eBPF collector (degraded state) | *(unset; set to any value to disable)* |
| `NETWATCH_DEV_DAEMON_ADDR` | Dev only: point the UI at a dev daemon; never read in packaged builds | `127.0.0.1:7897` |

## Folder Structure

**Today:**

```
net-watch-pro/
├── README.md            # this file
├── AGENTS.md            # working rulebook for AI assistants (read first)
├── PRD.md               # product requirements (scope, features, non-goals)
├── DESIGN.md            # UI design system (tokens, components, screens, a11y)
├── ARCHITECTURE.md      # stack, structure, data flow, naming, env vars
├── DATABASE.md          # schema, indexes, migrations, access rules
├── API.md               # endpoint reference, error format, versioning
└── REAPM.md             # reference-app teardown (background only, not binding)
```

**Planned** *(full tree in ARCHITECTURE.md §4)*:

```
├── crates/              # Cargo workspace: netwatchd, netwatch-core, netwatch-ebpf,
│                        #   netwatch-api, netwatch-common
├── app/                 # Tauri 2 shell (src-tauri/) + Svelte 5 frontend (src/)
├── xtask/               # build automation (BPF compile, packaging)
├── packaging/           # PKGBUILD, deb/, rpm/, systemd unit, license verifier
├── docs/                # api-contract.md, db-schema.md, env-vars.md, img/
└── tests/               # integration, accuracy harness, fixtures
```

## Available Scripts

**None exist yet** — the repository is documentation-only; there is no build system to script. Scripts will be defined when the workspace is scaffolded (ARCHITECTURE.md §4). Planned, per ARCHITECTURE.md §6 workflow: `cargo fmt` / `cargo clippy` for Rust, lint + `svelte-check` for the UI, and the `tests/accuracy/` harness for accounting verification. Do not use any command in this section until it appears in a real manifest.

## Contributing

1. Read `AGENTS.md` — it applies to human and AI contributors alike: follow the source-of-truth docs, keep them in sync with your change, stop and ask when a request conflicts with them.
2. Pick work that matches `PRD.md` scope (MVP §6.1; don't build V2/Later features early).
3. Small, reviewable diffs; Conventional Commits; tests for anything with a harness; a short summary of what changed and what you verified.
4. New dependencies, schema changes, new endpoints, and new UI primitives all require the corresponding doc update *in the same change* — see AGENTS.md §4.5/§5.

## License

Net Watch Pro is **dual-licensed**:

- **AGPL-3.0-or-later** — free for everyone. Use, study, modify, and share the code under the GNU Affero General Public License v3 or later; network-use of modified versions must offer the corresponding source (this product is local-only, so this clause mostly matters for forks that grow server-side features).
- **Commercial license** — available for contributors and users who cannot comply with AGPL terms (e.g. integrating Net Watch Pro into proprietary products or distributions). Contact the maintainer for terms and pricing.

*(Placeholder until the commercial-license terms document and contact channel exist — the choice between AGPL-only and dual-licensing was left open in PRD §10 OQ1 and is being resolved as dual for this README. Contributors keep copyright and license their contributions under both options.)*

---

*End of README.md. Accurate as of 2026-11-14 — documentation-phase state.*
