# Changelog

All notable changes to Net Watch Pro are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) ·
Versioning: SemVer with pre-release tags `vX.Y.Z-alpha.N` / `-beta.N` (PLAN.md §1).
Categories: Added / Changed / Fixed / Removed / Security.

---

## [v0.5.0-alpha.5] — 2026-11-18

Source-only alpha (no `.deb`/`.rpm`/AUR packages — packaging lands in PLAN
Phase 8). Tagged at current `main`: the Phase 5 Apps slice is complete and the
first seven Phase 6 freemium tasks are already merged. Phase 6 itself is **not**
complete — P6.8 (export menu + native save dialog) and P6.9 (usage budgets and
alerts) remain — so read this tag as "Phase 5 + freemium preview", not the
Phase 6 DoD release.

### Added

- **Apps slice (P5.1–P5.6):** A1 ranking, A2 detail + icon, A3 usage and A4
  endpoint APIs; three-tab App Detail screen with click-through routes from
  Dashboard and Live; cgroup-first attribution with honest confidence badges.
- **Freemium backend (P6.1–P6.5):** `daemon_events` migration and degraded-path
  event recording; offline Ed25519 license verifier (public key only in-repo);
  L1–L3 license endpoints storing only a key fingerprint; server-side Pro gating
  for C2 filters and the T2 retention ceiling; E1–E3 export engine producing
  CSV/JSON with 5/min creation and 10/min download limits.
- **Freemium UI (P6.6–P6.7):** license store with WS-driven state refresh, glass
  upgrade modal with §7.2 inline validation, and locked History filter chips
  that open the upgrade dialog for free-tier users.

### Changed

- Settings T1/T2 report tier-aware retention bounds (90 days free, 365 Pro).
- Rate-limit failures use `RATE_LIMITED` with `details.retry_after_s` and a
  `Retry-After` header.

### Fixed

- CI builds `netwatchd` before process-level tests; workspace Clippy is clean
  under `-D warnings`.

**Verification:** workspace tests 128 passed; Clippy 0 errors; UI `svelte-check`,
ESLint/Prettier and `vite build` clean.

**Known open items:** the upgrade modal's "Buy a license" target is a placeholder
until a storefront URL is defined; end-to-end activation against a live daemon
was not exercised in this release.

---

## [v0.0.1-alpha.1] — 2026-11-14

Phase 0 — Foundation & Tooling (PLAN.md §2). Documentation-phase → scaffolded
workspace; no product features yet.

### Added

- **Specification set (source of truth, priority order per AGENTS.md §2):**
  `PRD.md`, `DESIGN.md`, `ARCHITECTURE.md`, `DATABASE.md`, `API.md`, `REAPM.md`,
  plus working docs `AGENTS.md`, `PLAN.md`/`CHECKLIST.md`, `FEATURES.md`,
  `DATABASE.md`, `API.md`, `AUTH_SECURITY.md`, `MCP.md`, `INTEGRATIONS.md`,
  `TESTING.md`, `README.md`.
- **Cargo workspace** (`Cargo.toml`, `rust-toolchain.toml` @ 1.99.1, `deny.toml`,
  `clippy.toml`, `.editorconfig`, `.gitignore`) with five crate skeletons:
  `netwatch-common`, `netwatch-core`, `netwatchd` (binary stub, fail-fast),
  `netwatch-api`, `xtask` (P0.1, P0.2).
- **UI workspace** `app/`: Vite 7 + Svelte 5 (runes-only) + TypeScript strict
  (`noUncheckedIndexedAccess`) + ESLint flat + Prettier (P0.3).
- **Tauri 2 shell** `app/src-tauri/`: least-privilege capability
  (`core:default`), strict CSP, min window 960×600 (DESIGN.md §5.4), placeholder
  brand icons in primary teal; thin `main.rs` with explicit fail-fast startup
  (P0.4).
- **Token CSS layer** `app/src/lib/theme/tokens.css`: full DESIGN.md §3–§6
  transcription — light+dark palettes, chart series, type scale, 4px spacing,
  radius, shadows, motion (with `prefers-reduced-motion` collapse), layout
  variables (P0.6, CSS half).
- **CI** `.github/workflows/ci.yml`: Rust fmt/clippy/test + UI
  check/lint/test jobs with caching (P0.7).
- **Doc mirrors** `docs/api-contract.md`, `docs/db-schema.md`,
  `docs/env-vars.md` bound to `API.md`/`DATABASE.md`/`ARCHITECTURE.md` with a
  drift-fails-CI rule (P0.8).
- **Governance:** dual license (AGPL-3.0-or-later + commercial, `LICENSE`,
  `README.md` §License), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `SECURITY.md`, this `CHANGELOG.md`.

### Security

- Zero-outbound product contract documented and enforced by design from day one
  (`AUTH_SECURITY.md` §1): no accounts, no telemetry, no update pings.
- Token/licence secrets model defined before any code that could leak them.

---

## [v0.4.0-alpha.4] — 2026-11-17

First runnable alpha published on GitHub. This is a **source-only** alpha:
no `.deb`, `.rpm`, or AUR packages ship yet (packaging is PLAN Phase 8).
Build from source with Rust 1.99.1 and Node 20+. It includes the complete
Phase 4 dashboard slice plus the first Phase 5 backend task (P5.1); the
remaining App-detail work is still open.

### Added

- **Secure local transport (P1′):** root daemon serves a versioned JSON API and
  WebSocket stream on a Unix domain socket with `SO_PEERCRED` UID allowlist;
  Tauri shell bridges REST/WS to an unprivileged UI. No bearer token, no
  Polkit prompt per launch, and no network listeners (D1a).
- **eBPF spike (Phase S):** standalone `spikes/ebpf-accounting/` workspace with
  a nightly/BPF toolchain and `docs/spike-ebpf-report.md`, replacing guessed
  kernel claims with measured or explicitly tagged `[HYPOTHESIS]` statements.
- **Persistence and history (P2′/P3′):** forward-only SQLx migrations 0001–0003,
  batched connection writer, in-memory episode store, `live.connections`
  snapshot/delta channel, `GET /connections/live`, keyset-paginated
  `GET /connections`, `GET /connections/{id}`, and Live/History Svelte screens
  with loading, empty, and error states.
- **Lifecycle safety (P3.6–P3.7):** process-level restart-persistence test
  against the real daemon binary and idempotent soft delete of stale apps so
  history keeps resolving names.
- **Usage pipeline (P4.1–P4.5):** transactional raw-window compaction into
  15-minute rollups with a 50-endpoint/day cap, periodic interface-counter
  sampler (`/sys/class/net/*/statistics`, `lo` excluded), and REST endpoints
  `GET /usage/summary`, `GET /usage/timeline`, and `GET /usage/interfaces`
  using IANA timezone and billing-cycle boundaries (D3a/D4a).
- **Dashboard UI (P4.6–P4.9):** token-derived ECharts theme, reusable themed
  chart wrapper, full dashboard with headline cards, Day/Hour timeline, top-app
  bars, Unattributed row, interface breakdown, `usage.totals` streaming, and
  honest “Waiting for first data” empty states.
- **Apps API (P5.1):** `GET /apps` ranking with `sort`, `order`, `limit`, and
  `include_deleted`, returning A1-shaped usage rows; ranking matches U1 top
  apps.
- **Starship Control docs (Phase 12):** planned read-only bridge at
  `$XDG_RUNTIME_DIR/starship/bridges/nwp.sock` and plugin manifest location.

### Changed

- **API.md 1.2.1:** soft-deleted apps are excluded from U1 `top_apps` so A1 and
  the dashboard ranking agree.
- Authentication model changed from token file to socket peer credentials;
  `NETWATCH_API_TOKEN_FILE` and the Polkit token helper are retired.
- All chart colors, fonts, radii, and shadows come from `DESIGN.md` tokens;
  ECharts imports are tree-shaken to line/bar + grid/tooltip/legend/canvas.
- The old static dashboard placeholder was replaced by the real data-driven
  screen; History now reuses the shared byte formatter.

### Fixed

- Clippy is clean under `-D warnings` across the whole workspace (previously
  94 errors, including the reported `unnecessary_debug_formatting` in
  `netwatch-api`).
- CI now builds `netwatchd` explicitly before `cargo test --workspace` so the
  process-level restart test can spawn the binary.
- Attribute parser now reads the contents of `/proc/<pid>/cgroup` rather than
  the path string; cgroup suffix parsing is case-insensitive and lossless.
- Doc/mirror drift on auth, transport, rollups, capabilities, and certainty
  tagging was resolved across source-of-truth files.

### Security

- No outbound product traffic: no telemetry, analytics, update pings, or CDN
  fonts. All data stays in the local SQLite database.
- Authorization remains socket-level peer credentials; no secret is stored in
  code, tests, or commits.

**Verification:** GitHub Actions `main` CI green (`fmt`, `clippy -D warnings`,
workspace tests, UI check/lint/test); full workspace suite passed with 95 Rust
tests; UI `svelte-check` reported 0 errors/warnings.

---

[Unreleased]: https://github.com/zvirox/net-watch-pro/compare/v0.5.0-alpha.5...HEAD
[v0.5.0-alpha.5]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.5.0-alpha.5
[v0.4.0-alpha.4]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.4.0-alpha.4
[v0.0.1-alpha.1]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.0.1-alpha.1
