# SETUP.md — Run Net Watch Pro Locally

**Version:** 1.1 · **Date:** 2026-11-15

Exact steps to get the repository building on your machine. Every command
below was verified against this repository's real `package.json`,
`tauri.conf.json`, `Cargo.toml`, and CI workflow.

> **Read this first:** Net Watch Pro is **pre-alpha, Linux-only** (see status
> in [README.md](README.md)). The daemon does not monitor traffic yet and
> there is no database to initialize — those arrive in Phases S/D/1'–2'
> ([PLAN.md](PLAN.md)). This guide gets the Phase 0 scaffold building and
> running, which is everything that exists today.

---

## 1. Prerequisites

| Tool | Version | Why | Check |
|---|---|---|---|
| Linux | Arch/CachyOS or another current distro | The product targets Linux only (PRD §3). There are no Windows/macOS steps in this guide — see the note at the end. | — |
| Rust | 1.99.1 (MSRV, pinned in `rust-toolchain.toml`) | Workspace, daemon, Tauri shell | `rustc --version` |
| Node.js | 20+ (CI uses 22) | UI workspace (Vite 7 + Svelte 5) | `node --version` |
| npm | 10+ (ships with Node) | UI package manager | `npm --version` |
| WebKit2GTK 4.1 | any current | Tauri 2 runtime library | `pkg-config --exists webkit2gtk-4.1 && echo ok` |
| GTK4 | any current | Tauri 2 runtime library | `pkg-config --exists gtk4 && echo ok` |

No Python, no database server, no Docker: SQLite is bundled by the daemon
(DATABASE.md §1) and the accuracy tooling that uses Python lands in a later
phase.

**Install the system libraries** (Tauri 2 cannot build without them):

~~~bash
# Arch / CachyOS
sudo pacman -S --needed webkit2gtk-4.1 gtk4 pkgconf
~~~

*(Debian/Ubuntu-style `libwebkit2gtk-4.1-dev` and Fedora `webkit2gtk4.1-devel`
commands will be added when those distros are actually tested — CONTRIBUTING.md
tracks this.)*

## 2. Clone the repository

~~~bash
git clone https://github.com/zvirox/net-watch-pro.git
cd net-watch-pro
~~~

## 3. Install dependencies

### Rust workspace (no external crates yet — fast)

~~~bash
cargo check --workspace
~~~

### UI workspace

~~~bash
cd app
npm install --no-audit --no-fund
cd ..
~~~

This creates `app/node_modules/` and `app/package-lock.json` from the pinned
dev dependencies in `app/package.json`. Expect it to take a few minutes; on a
slow registry, re-running the same command resumes from npm's cache.

## 4. Environment file

~~~bash
cp .env.example .env   # optional — defaults work without it
~~~

`.env.example` mirrors the closed variable list in `ARCHITECTURE.md §8.3` /
`docs/env-vars.md` (`NETWATCH_DATA_DIR`, `NETWATCH_API_ADDR`, …). **Nothing
reads `.env` yet** — the daemon lands in Phase 1 (PLAN.md P1.1). Do not add
variables to this file; the list is closed by governance (AGENTS.md §5).

## 5. Database setup

**Nothing to do.** There are no migrations yet. SQLite is embedded (WAL
mode) and the schema ships inside the daemon as embedded SQL migrations
(`crates/netwatchd/src/db/migrations/`, applied automatically at boot when
Phase 2 lands). Never run ad-hoc SQL against a production database file
(DATABASE.md §6).

## 6. Run the dev servers

Two terminals, from the repository root:

~~~bash
# Terminal 1 — UI dev server (strict port 5173, from app/vite.config.ts)
cd app && npm run dev
~~~

Open **http://127.0.0.1:5173** in a browser — you'll see the Phase 0 smoke
UI with the vendored Button components.

~~~bash
# Terminal 2 — desktop window (loads the Vite dev URL from tauri.conf.json)
cargo run -p netwatch-app
~~~

The Tauri window ("Net Watch Pro", min 960×600 per DESIGN.md §5.4) attaches
to the Vite server. `cargo run -p netwatch-app` without Terminal 1 also
launches, but the webview may show an error page — start Vite first.

The daemon binary is a skeleton until P1.1; running it just prints its phase
banner and exits:

~~~bash
cargo run -p netwatchd
# → "netwatchd skeleton — Phase 0 (P0.2); real boot lands in P1.1"
~~~

Production-style `tauri dev`, the systemd service, and the localhost API do
not exist yet.

## 7. Run the checks (the local gate)

From the repository root:

~~~bash
# Rust: format, lint (deny warnings), tests
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
~~~

From `app/`:

~~~bash
cd app
npm run check    # svelte-check, strict TS
npm run lint     # eslint + prettier --check
npm run test     # placeholder until the first component test lands
npm run build    # production bundle → app/dist
~~~

CI (`.github/workflows/ci.yml`) runs exactly these. All four Rust commands
and every `npm run *` script above exist in the real manifests.

---

## 8. Common setup errors and fixes

1. **`error: failed to open icon …/app/src-tauri/icons/icon.png: No such file or directory`**
   Tauri's build script requires the bundled icons. They are committed, so
   this means a dirty tree or deleted icons. Restore them:
   `git checkout -- app/src-tauri/icons/` (or `git stash list` first if you
   deleted them deliberately).

2. **`The system library 'webkit2gtk-4.1' required by crate 'tauri' not found`** (or any
   `pkg-config` failure on `webkit2gtk-4.1` / `gtk4`)
   The system libraries are missing. Install them (`§1`):
   `sudo pacman -S --needed webkit2gtk-4.1 gtk4 pkgconf`, then retry
   `cargo check -p netwatch-app`.

3. **`npm install` is extremely slow or times out**
   A slow registry makes the resolution phase take many minutes with no
   visible output — that is normal here, not a hang. Re-run with the warm
   cache: `npm install --prefer-offline --no-audit --no-fund`. Check reachability first with
   `npm ping` (~1 s round-trip is the observed baseline on this network).

4. **`ERESOLVE overriding peer dependency` / `peer typescript@">=4.8.4 <6.1.0"` warnings**
   Non-fatal npm resolution warnings about the TypeScript peer range.
   The install completes; do not "fix" this by switching TypeScript
   versions — the pin (`^5.9.4`) is deliberate.

5. **Vite exits immediately: `Port 5173 is already in use`**
   `app/vite.config.ts` sets `strictPort: true` — Vite refuses to fall back
   to another port. Free it: `ss -ltnp 'sport = :5173'` then kill the listed
   process, or stop whatever is occupying the port.

6. **`error: toolchain '1.99.1' is not installed`** (rustup-managed machines)
   The repo pins the MSRV via `rust-toolchain.toml`. Install it:
   `rustup toolchain install 1.99.1`. On distro-packaged Rust (no rustup)
   the file is inert documentation; if your distro Rust is older, install
   rustup instead of overriding the pin.

7. **`cargo clippy … -- -D warnings` fails on your change**
   Warnings are errors by policy (ARCHITECTURE.md §8.2). Run
   `cargo fmt` first, then fix the reported lint. Do not add `#[allow]`
   without a comment explaining why.

8. **`npm run check` fails with `svelte-check: command not found` (or similar)**
   Dependencies were never installed (or `node_modules` was wiped). Run
   `cd app && npm install` first; all `npm run *` scripts depend on it.

9. **Window is blank when launched with `cargo run -p netwatch-app`**
   The webview loads the dev server URL — Vite must be running first
   (`§6`, Terminal 1). A blank window with Vite running means the smoke
   build is stale: `cd app && npm run build` and restart the app.

10. **`cannot open display` / GTK crash launching the window**
    Tauri needs a desktop session. Run inside your graphical session (not
    over bare SSH). Over SSH, use your machine directly or forward with
    `ssh -X` and ensure Wayland/X11 access.

11. **Trying to build on Windows or macOS**
    Not supported and not accidental: the daemon is a Linux systemd service
    using eBPF (PRD §3). CI builds Linux only. WSL2 also lacks the eBPF
    surface this project needs.

12. **You added a dependency or env var and CI/governance flags it**
    New npm packages, crates, and environment variables require maintainer
    approval and a doc update *in the same change* (AGENTS.md §5,
    ARCHITECTURE.md §8.3). Revert and propose it first.

---

## 9. Platform notes

- **Linux (supported):** everything in this guide. Tested on Arch/CachyOS
  with rustc 1.99.1, Node 26, WebKit2GTK 2.54, GTK 4.24.
- **Windows / macOS:** unsupported. The daemon depends on Linux eBPF and a
  systemd service; the UI alone would build on macOS with extra effort, but
  no one has tried and it is out of MVP scope (PRD §3).

---

*End of SETUP.md. Accurate as of 2026-11-14, Phase 0 (v0.0.1-alpha.1).*
