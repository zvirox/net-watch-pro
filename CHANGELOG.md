# Changelog

All notable changes to Net Watch Pro are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) ·
Versioning: SemVer with pre-release tags `vX.Y.Z-alpha.N` / `-beta.N` (PLAN.md §1).
Categories: Added / Changed / Fixed / Removed / Security.

---

## [v0.8.8-alpha.8] — 2026-11-19

Patch release: fixes the UI stuck in "Connecting…" after onboarding.

### Fixed

- `daemon_connect` is now idempotent. Onboarding calls `authorize()` after the
  app already connected; previously that returned "already connected" and every
  subsequent reconnect failed, leaving the status chip on "Connecting…" and
  preventing settings/history from loading.

---

## [v0.8.7-alpha.8] — 2026-11-19

Patch release: fixes all UI→daemon REST calls from the installed app.

### Fixed

- The Tauri bridge was stripping `/api/v1` before sending requests, so every
  REST call (including onboarding's health check) returned 404. The full
  versioned path is now sent unchanged, and percent-encoded query strings are
  preserved and validated.

---

## [v0.8.6-alpha.8] — 2026-11-19

Patch release: daemon can set its socket group under the hardened unit.

### Fixed

- Added `CAP_CHOWN` to the unit's capability bounding set. Without it the
  daemon failed at startup with `cannot set socket group: Operation not
  permitted` and exited with status 5, so onboarding never became healthy.

---

## [v0.8.5-alpha.8] — 2026-11-19

Patch release: production binaries now embed the UI correctly.

### Fixed

- Release builds enable Tauri's `custom-protocol` feature. Without it, plain
  `cargo build --release` produced a dev-mode binary that tried to load the
  Vite dev URL (`127.0.0.1:5173`) and showed "Connection refused" even after a
  successful install. Applied to both the AUR PKGBUILD and release workflow.

---

## [v0.8.4-alpha.8] — 2026-11-19

Patch release: AUR builds now clear all makepkg toolchain flags for Rust.

### Fixed

- AUR `PKGBUILD` clears `CFLAGS`/`CXXFLAGS`/`LDFLAGS` in `build()`, not just
  `LDFLAGS`. makepkg's `-fno-plt`/`--as-needed` flags were breaking bundled
  SQLite in `sqlx_macros` (`undefined symbol: sqlite3_free` on CachyOS).

---

## [v0.8.3-alpha.8] — 2026-11-19

Patch release: fixes AUR builds on CachyOS/Arch.

### Fixed

- AUR `PKGBUILD` now clears makepkg's `LDFLAGS` for the Rust build;
  `-Wl,--as-needed` was dropping bundled SQLite symbols from `sqlx_macros`
  and caused `undefined symbol: sqlite3_free`.

---

## [v0.8.2-alpha.8] — 2026-11-19

Patch release: fixes the AUR source bundle checksum.

### Fixed

- Regenerated `PKGBUILD`/`.SRCINFO` sha256 for `net-watch-pro-git.install`
  after the P8.7 uninstall changes, so `makepkg` source validation passes.
- `makepkg --nobuild --nodeps` now validates all sources successfully.

---

## [v0.8.1-alpha.8] — 2026-11-19

Patch release: onboarding now makes the API socket accessible immediately.

### Fixed

- The one-time pkexec step now sets `NETWATCH_API_GID` to the desktop user's
  primary group (and adds the user to the `netwatch` group), so the `0660`
  socket can be reached without logging out and back in. The allowed-UID list
  remains the peer-credential authorization boundary.
- No other behavior changed; packages and signatures are rebuilt from this tag.

---

## [v0.8.0-alpha.8] — 2026-11-19

First installable alpha: `.deb`, `.rpm` and AUR `net-watch-pro-git` sources are
published with GPG-signed release assets. Still a source/build-level alpha —
hosted apt/dnf repositories remain deferred until after beta (PLAN F-10).

### Added

- **Hardened systemd unit (P8.1):** sandboxing, capability bounding and
  systemd-managed state/runtime directories; dev unit kept separately.
- **Debian/RPM/AUR packages (P8.2–P8.4):** maintainer scripts create the
  `netwatch` socket group and GID drop-in, leave the unit disabled for
  onboarding, and share one desktop entry/unit definition.
- **Onboarding (P8.5):** first-run gate with a single pkexec prompt that
  registers the desktop UID and enables the daemon, plus health polling.
- **Failure path + demo mode (P8.6):** actionable onboarding guidance, retry,
  and a read-only in-memory demo with a persistent Demo mode badge (OQ4).
- **Uninstall contract (P8.7):** service files removed and data deleted by
  default; `/etc/netwatch-pro/keep-data` opts in to retaining data.
- **Release workflow (P8.8):** tag-triggered build, detached GPG signatures for
  every artifact plus `SHA256SUMS`, public key asset, and a clean-runner deb
  install/uninstall smoke test.

### Changed / fixed

- The API socket is now created `0660` owned by the desktop group via
  `NETWATCH_API_GID` (with `SUDO_GID` fallback), instead of root-only `0755`.

### Verification

Workspace tests 141 passed, Clippy clean; UI checks clean; RPM built and
inspected; uninstall harness 6/6; systemd security score 2.9 OK; package
signing verified with a locally generated test key.

### Known open items

- eBPF attach remains blocked by kernel `-EPERM`; the daemon uses `/sys`
  interface counters and degraded events.
- Live pkexec/systemd, GUI tray and clean-VM installs were not exercised in
  this environment; they are covered by CI once this tag is pushed.

---

## [v0.7.0-alpha.7] — 2026-11-18

Source-only alpha completing the Phase 7 system slice: retention, settings,
interfaces, tray and persisted theming.

### Added

- **Retention pruner (P7.1):** core cutoff/tier math, an hourly daemon task with
  a first pass at boot, deletion of expired `connections` and `usage_endpoint_day`
  rows, soft-delete of stale apps, and preserved rollups. The configured value is
  re-clamped to 90/365 days against the current license on every pass.
- **Settings screen (P7.3):** keyboard-navigable §8.7 tabs for General, Data,
  Budgets, Interfaces, Daemon, License and About, plus a token-only Switch
  primitive (§7.13).
- **Interfaces I1/I2 + UI (P7.4):** list all interfaces with U3 bytes and flags,
  validated boolean PATCH with 404/400 handling, a status broadcast on exclusion
  changes, and a Settings tab with Metered/Excluded switches that refresh the
  dashboard.
- **Tray (P7.5):** native menu with Open, Pause UI notifications and Quit (UI
  only — the root daemon keeps running); today's usage pushed to the tooltip from
  the existing `usage.totals` feed.
- **Theme (P7.6):** persisted `theme_default` applied at app start, OS dark-mode
  following in System mode, and ECharts re-theming via `data-theme` observation.
- **Settings plumbing (P7.7):** `metered_policy` applied to newly discovered
  interfaces (`label` marks metered; `ask`/`auto` leave manual labeling in MVP),
  and `start_on_boot` synced by the root daemon to `systemctl enable/disable`
  with change-only polling and degraded-event reporting.

### Verification

Workspace tests 141 passed, 0 failed; Clippy `-D warnings` clean; Tauri shell
builds with tray, dialog and notification features/capabilities; UI
`svelte-check`, ESLint/Prettier and `vite build` clean.

### Known open items

- eBPF attach remains blocked by kernel `-EPERM` (spike report); the daemon uses
  `/sys` interface counters and degraded events.
- Live-daemon checks (systemd sync, tray rendering, toggle → dashboard update)
  were not run in this environment; verified by contract tests and compilation.

---

## [v0.6.0-alpha.6] — 2026-11-18

Source-only alpha completing the Phase 6 freemium slice. The full loop is now
implemented: activate offline → Pro unlocks without restart → filters, exports
and retention unlock; deactivate → locks return. No outbound product traffic.

### Added

- **Offline licensing (P6.1–P6.4):** `daemon_events` migration and degraded-path
  recording; Ed25519 verifier with only the public key and a SHA-256 fingerprint
  in-repo; L1–L3 license endpoints; server-side Pro gating for C2 filters and
  the T2 retention ceiling.
- **Exports (P6.5, P6.8):** E1–E3 in-memory jobs producing CSV/JSON with 5/min
  creation and 10/min download limits, `RATE_LIMITED` details + `Retry-After`,
  deactivation grace for started jobs, and a History topbar menu that saves via
  the native Tauri dialog.
- **Freemium UI (P6.6–P6.7):** WS-driven license store, glass upgrade modal with
  inline validation, and locked History filter chips with tooltips.
- **Budgets & alerts (P6.9):** pure core evaluation with warning/breach levels,
  five free-tier budget settings keys, metered-aware budget state on U1, a
  Settings screen with usage bars, and deduped desktop notifications via the
  official Tauri notification plugin.

### Changed

- U1 gains an additive `budgets` object (top-10 per-app states); settings gains
  `budget_*` keys in whole MB with `0` meaning disabled.
- Removed the unused `reqwest` declaration from the Tauri shell so the
  zero-outbound claim is enforceable by the dependency graph.
- Plan tracker reconciled against verified code; kernel-blocked eBPF items are
  marked blocked rather than done.

### Verification

Workspace tests 134 passed, 0 failed; Clippy `-D warnings` clean; Tauri shell
builds with both plugins and their least-privilege capabilities; UI
`svelte-check`, ESLint/Prettier and `vite build` clean. Zero outbound calls
audited in product code (no fetch/XHR/CSS CDN; no daemon HTTP client).

### Known open items

- eBPF attach remains blocked by kernel `-EPERM` (spike report); the daemon uses
  `/sys` interface counters and records degraded events.
- End-to-end notification delivery and live-daemon activation were not run; they
  are verified by contract tests, capability generation and code review.
- Phase tags `v0.1.0-alpha.1` through `v0.3.0-alpha.3` were never published.

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

[Unreleased]: https://github.com/zvirox/net-watch-pro/compare/v0.8.8-alpha.8...HEAD
[v0.8.8-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.8-alpha.8
[v0.8.7-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.7-alpha.8
[v0.8.6-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.6-alpha.8
[v0.8.5-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.5-alpha.8
[v0.8.4-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.4-alpha.8
[v0.8.3-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.3-alpha.8
[v0.8.2-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.2-alpha.8
[v0.8.1-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.1-alpha.8
[v0.8.0-alpha.8]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.8.0-alpha.8
[v0.7.0-alpha.7]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.7.0-alpha.7
[v0.6.0-alpha.6]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.6.0-alpha.6
[v0.5.0-alpha.5]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.5.0-alpha.5
[v0.4.0-alpha.4]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.4.0-alpha.4
[v0.0.1-alpha.1]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.0.1-alpha.1
