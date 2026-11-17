<p align="center">
  <img src="docs/assets/net-watch-pro-logo.png" alt="Net Watch Pro logo" width="220" />
</p>

<h1 align="center">Net Watch Pro</h1>

<p align="center">
  <strong>A local-first Linux network-traffic monitor and per-app data-usage tracker, built with Rust, Tauri 2, Svelte 5, and Aya eBPF.</strong>
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
  <a href="FEATURES.md">Feature build order</a> ·
  <a href="#development-setup">Development</a> ·
  <a href="#contributing">Contributing</a>
</p>

## Project status

Net Watch Pro is **pre-alpha**. The repository currently contains the complete
specification set (see [Project plan](PLAN.md) and the docs listed there) plus
the Phase 0 foundation: a six-crate Cargo workspace, the Tauri 2 desktop
shell, the UI workspace with the vendored design-system primitives, and CI.
**No traffic monitoring exists yet** — the daemon observes nothing and the UI
shows no data. Phase tracking, with per-task verification evidence, lives in
the [progress checklist](CHECKLIST.md).

**Current work (2026-11-15):** the planning docs are being revised per
`PLAN-FIX-INSTRUCTIONS.md` findings F-01–F-11 (eBPF spike inserted as Phase S,
Unix-socket + peer-credential transport replacing loopback+token, endpoint-day
aggregates for long-term history, interface-counter headline totals, and
certainty tagging of unmeasured kernel claims). Owner decisions D1–D7 are
recorded in `docs/decisions.md`. No product code is being written during this
fix effort except inside `spikes/`.

The [changelog](CHANGELOG.md) records what each tagged release contains. The
first tagged release, `v0.0.1-alpha.1`, is the Phase 0 scaffold: it compiles,
passes fmt/clippy/check, and ships no product features.

## What works in the current checkout

- The workspace builds and passes the local gate: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` are green.
- Five daemon-side crate skeletons (`netwatch-common`, `netwatch-core`,
  `netwatchd`, `netwatch-api`, `xtask`) compile with zero external
  dependencies and the workspace-wide lint set (`deny(unsafe_code)`,
  `clippy::pedantic`, no `unwrap`/`expect` outside startup).
- The Tauri 2 shell (`app/src-tauri`) compiles against system WebKit2GTK with
  a least-privilege capability (`core:default`) and a strict CSP. It opens a
  window showing a smoke UI; no daemon link yet.
- The UI workspace (`app/`) runs Svelte 5 (runes-only), strict TypeScript,
  ESLint flat config, and Prettier, with `npm run check` / `npm run lint`.
- The DESIGN.md token system is implemented as CSS custom properties
  (`app/src/lib/theme/tokens.css`): light and dark themes, chart palette,
  type scale, spacing, radius, shadows, motion, with self-hosted Inter
  Variable and JetBrains Mono — no font CDN fetches.
- Vendored shadcn-svelte-style primitives (`button`, `input`, `table`) styled
  strictly from those tokens; Tailwind is intentionally not a dependency.
- CI (`.github/workflows/ci.yml`) runs the Rust and UI gates on every push
  and pull request; the doc mirrors in `docs/` are the drift checks for
  `API.md`, `DATABASE.md`, and the environment-variable list.

## Known limitations and work in progress

- **No monitoring yet.** The eBPF collector, attribution, accounting, SQLite
  persistence, and the localhost API arrive in Phases 1–3 (see
  [PLAN.md](PLAN.md)). The UI's only screen is a Phase 0 smoke stub.
- No packages exist. Arch, deb, and rpm packaging is planned for Phase 8;
  until then the app runs only from a development checkout.
- No license/Pro functionality, no exports, no history — freemium gates land
  after the core monitoring slice (see [FEATURES.md](FEATURES.md)).
- The reference teardown in `REAPM.md` documents a *different* product and is
  background material only.
- Firewall features are **out of scope until after V1.0** (PRD §12); the
  monitor-first boundary is a product promise, not a roadmap shortcut.

## Architecture

~~~mermaid
flowchart LR
  UI["Svelte 5 UI (unprivileged)"]
  API["netwatch-api - Unix socket, peer credentials"]
  Core["netwatch-core - accounting, attribution, retention"]
  EBPF["Aya eBPF - observe only"]
  DB[("SQLite - connections, rollups, events")]

  UI -->|WS and REST over Unix socket| API
  API --> Core
  EBPF --> Core
  Core --> DB
  DB --> API
~~~

Two processes, one boundary: a root systemd daemon (`netwatchd`) observes
traffic with eBPF and writes local SQLite; the unprivileged Tauri UI talks
**only** to the peer-credential-authorized local API on a Unix domain socket.
There is no bearer token, no cloud, no telemetry, and no outbound connection —
by contract, not by accident ([AUTH_SECURITY.md](AUTH_SECURITY.md)). Details in
[ARCHITECTURE.md](ARCHITECTURE.md).

## Install and try it

**There are no installable packages yet.** Pre-release builds will appear on
the [releases page](https://github.com/zvirox/net-watch-pro/releases) once
packaging lands (Phase 8, per the [plan](PLAN.md)). Do not install anything
claiming to be Net Watch Pro before then.

## Development setup

### Requirements

- Linux with a current Rust toolchain (MSRV 1.99.1, pinned in
  `rust-toolchain.toml`) and Node.js 20+.
- Tauri 2 development dependencies: `webkit2gtk-4.1`, `gtk4`, and friends —
  see [CONTRIBUTING.md](CONTRIBUTING.md) for per-distro package lists.
- Root access is **not** needed for development; the daemon stub runs
  unprivileged.

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
npm run check    # svelte-check (strict TS)
npm run lint
~~~

The CI workflow runs exactly these gates. Behaviour that CI cannot verify
(eBPF attach, root-privileged flows, packaged installs) is re-checked
manually per [TESTING.md](TESTING.md) before each release.

### Documentation-first rule

The specs are the contract, in priority order: `PRD.md` → `DESIGN.md` →
`ARCHITECTURE.md` → `DATABASE.md` → `API.md`. Code that drifts from them is
wrong. [AGENTS.md](AGENTS.md) is the binding rulebook for human and AI
contributors alike.

## Releases and feedback

Release gates, tagging, and the alpha→beta→1.0 cadence are documented in
[PLAN.md](PLAN.md); tagged changes live in the
[changelog](CHANGELOG.md).

Alpha feedback should include the release version, distribution, desktop
session, and reproduction steps — report it through the
[GitHub issue tracker](https://github.com/zvirox/net-watch-pro/issues). For
security issues, follow the private disclosure process in
[SECURITY.md](SECURITY.md); never open a public issue for those.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a change. In short:
plan first (name the governing doc sections), keep diffs small and scoped to
one task, ship tests with the code ([TESTING.md](TESTING.md) §8 gate), sync
the affected docs in the same change, and use Conventional Commits. New
dependencies require maintainer approval first.

### Where help is most useful

| Area | Examples | Difficulty |
|---|---|---|
| Testing and bug reports | Run the gates on your distro and report what breaks | Easy |
| Documentation | Clarify setup, specs, and workflows | Easy |
| UI polish | Missing states, a11y fixes, keyboard shortcuts | Easy to medium |
| Test coverage | Accounting/attribution accuracy tests (`tests/accuracy/`) | Medium |
| Packaging | Arch, deb, rpm pipelines and verification (Phase 8) | Medium |
| eBPF / kernel | Aya attach targets, kernel-version quirks | Hard |

## License

Net Watch Pro is **dual-licensed**:

- **AGPL-3.0-or-later** — free to use, study, modify, and share under the GNU
  Affero General Public License v3 or later ([LICENSE](LICENSE)).
- **Commercial license** — for contributors and users who cannot comply with
  AGPL terms (e.g. embedding Net Watch Pro in proprietary products or
  distributions). Contact the maintainer for terms.

By contributing you agree that your work is released under both options.
