# CHECKLIST.md — Plan Tracker

**Companion to `PLAN.md`** (task details: what/files/verify/deps live there; this file is for ticking). Rules: tick only verified work; a phase is done when its DoD box is ticked; release tags gate on the phase DoD.

**Legend:** `[ ]` open · `[x]` done · `[~]` in progress · `[!]` blocked (note why inline)

---

## Phase 0 — Foundation & Tooling → `v0.0.1-alpha.1` (week 1)

- [ ] P0.1 Cargo workspace root configs
- [ ] P0.2 Crate skeletons (common, core, netwatchd, api, xtask)
- [ ] P0.3 UI workspace (Vite + Svelte 5 + TS strict + lint)
- [ ] P0.4 Tauri 2 shell, window opens, capabilities least-privilege
- [ ] P0.5 shadcn-svelte vendored: button, input, table
- [ ] P0.6 Token CSS + self-hosted fonts, both themes
- [ ] P0.7 CI: fmt/clippy/svelte-check + 3-distro build matrix
- [ ] P0.8 Docs mirrors (api-contract, db-schema, env-vars)
- [ ] **DoD: themed blank window runs; `cargo check` + CI green**

## Phase 1 — Walking Skeleton → `v0.1.0-alpha.1` (week 2)

- [ ] P1.1 Token file + loopback API bind
- [ ] P1.2 `/health` + `/status`
- [ ] P1.3 Error envelope + status mapping
- [ ] P1.4 WS stream skeleton (subscribe/ping)
- [ ] P1.5 UI WS client + reconnect backoff
- [ ] P1.6 Status chip (4 states) on `status` channel
- [ ] P1.7 Dashboard screen shell + app shell (sidebar/topbar)
- [ ] P1.8 Dev systemd unit
- [ ] **DoD: UI ↔ daemon connected; reconnect ≤ 5 s; tag `v0.1.0-alpha.1`**

## Phase 2 — Live Slice → `v0.2.0-alpha.2` (week 3)

- [ ] P2.1 eBPF attach + ring buffer plumbing
- [ ] P2.2 Byte counters → 1 s delta events
- [ ] P2.3 `/proc` attribution + `apps` upserts
- [ ] P2.4 Migrations 0001+0002 at boot
- [ ] P2.5 Batched `connections` writer
- [ ] P2.6 `live.connections` channel (snapshot + delta)
- [ ] P2.7 Live Connections screen (virtualized, pause, all states)
- [ ] P2.8 REST `/connections/live` + real ebpf status
- [ ] P2.9 Degraded path (`NETWATCH_DISABLE_EBPF` → honest UI)
- [ ] **DoD: real traffic visible, app-labeled, pause/sort work; tag `v0.2.0-alpha.2`**

## Phase 3 — History Slice → `v0.3.0-alpha.3` (week 4)

- [ ] P3.1 Migration 0003: rollups + remote_host index
- [ ] P3.2 `GET /connections` (keyset pagination, basic filters)
- [ ] P3.3 Pro-advanced-filter 403 gate (locked off)
- [ ] P3.4 `GET /connections/{id}` (+ pruned 404 wording)
- [ ] P3.5 History screen (search, table, all states)
- [ ] P3.6 Restart persistence test
- [ ] P3.7 `apps` soft delete on last activity
- [ ] **DoD: restart keeps history; free history search works; tag `v0.3.0-alpha.3`**

## Phase 4 — Dashboard Slice → `v0.4.0-alpha.4` (week 5)

- [ ] P4.1 Compaction job (raw → rollups at 7 days)
- [ ] P4.2 `GET /usage/summary`
- [ ] P4.3 `GET /usage/timeline` (raw+rollup union)
- [ ] P4.4 `GET /usage/interfaces`
- [ ] P4.5 ECharts token theme wrapper
- [ ] P4.6 Dashboard screen (headline, timeline, top apps, interfaces)
- [ ] P4.7 `usage.totals` WS → headline cards
- [ ] P4.8 Dashboard empty state
- [ ] **DoD: totals ±5% vs interface counters; tag `v0.4.0-alpha.4`**

## Phase 5 — Apps Slice → `v0.5.0-alpha.5` (week 6)

- [ ] P5.1 `GET /apps` (ranking, include_deleted)
- [ ] P5.2 `GET /apps/{id}` + icon resource
- [ ] P5.3 `GET /apps/{id}/usage` + `/endpoints`
- [ ] P5.4 App Detail screen (3 tabs)
- [ ] P5.5 Click-through routes from Dashboard/Live
- [ ] P5.6 Attribution edge cases + confidence display
- [ ] **DoD: every app opens a meaningful detail page; tag `v0.5.0-alpha.5`**

## Phase 6 — Freemium Slice → `v0.6.0-alpha.6` (week 7)

- [ ] P6.1 Migration 0004: `daemon_events`
- [ ] P6.2 License verifier (offline signature, no network)
- [ ] P6.3 License endpoints (get/activate/deactivate)
- [ ] P6.4 Server-side Pro gating (filters, retention ceiling)
- [ ] P6.5 Export engine (job → CSV/JSON, rate limits)
- [ ] P6.6 Upgrade modal + license states
- [ ] P6.7 History Pro locks (chips + tooltips)
- [ ] P6.8 Export menu + native save dialog
- [ ] **DoD: activate → instant unlock; deactivate → locks return; zero network calls; tag `v0.6.0-alpha.6`**

## Phase 7 — System Slice → `v0.7.0-alpha.7` (week 8)

- [ ] P7.1 Retention pruner (free/Pro aware)
- [ ] P7.2 `GET/PUT /settings` (typed, bounded)
- [ ] P7.3 Settings screen (general/data/daemon)
- [ ] P7.4 Interfaces + metered labeling (API + UI)
- [ ] P7.5 Tray icon + menu (quit-not-daemon)
- [ ] P7.6 Theme switching, persisted, charts re-theme
- [ ] P7.7 metered_policy + start_on_boot plumbed
- [ ] **DoD: tray quit keeps daemon; retention correct per tier; tag `v0.7.0-alpha.7`**

## Phase 8 — Packaging Slice → `v0.8.0-alpha.8` (week 9) — *first public alpha*

- [ ] P8.1 Hardened systemd unit
- [ ] P8.2 `.deb` package + post-install
- [ ] P8.3 `.rpm` parity
- [ ] P8.4 AUR `net-watch-pro-git`
- [ ] P8.5 Onboarding flow (pkexec, health checks, 3 cards, < 3 min)
- [ ] P8.6 Failure path + demo-mode decision (PRD OQ4)
- [ ] P8.7 Uninstall completeness
- [ ] P8.8 Repo signing + release workflow
- [ ] **DoD: installable via repo artifacts on all 3 distro families; tag `v0.8.0-alpha.8`**

## Phase 9 — Diagnostics & Accessibility → `v0.9.0-beta.1` (week 10) — *public beta*

- [ ] P9.1 Diagnostics endpoint + modal
- [ ] P9.2 All degraded states end-to-end
- [ ] P9.3 Daemon-stopped banner + recovery actions
- [ ] P9.4 Contrast audit (both themes)
- [ ] P9.5 Keyboard nav + focus traps + SR labels
- [ ] P9.6 Chart data-table fallbacks
- [ ] P9.7 Reduced motion + 200% zoom
- [ ] **DoD: PRD DoD #3/#5/#6 pass; zero a11y blockers; tag `v0.9.0-beta.1`**

## Phase 10 — Beta Hardening → `v0.9.x-beta.N` (weeks 11–12)

- [ ] P10.1 Accuracy harness ±5% in CI
- [ ] P10.2 Perf budget test (≤3% CPU / ≤150 MB RSS p95)
- [ ] P10.3 1-year retention soak (< 1.5 GB)
- [ ] P10.4 3-distro CI matrix full suite
- [ ] P10.5 Security review closed
- [ ] P10.6 Beta feedback fixes (patch-tagged)
- [ ] **DoD: PRD §11 DoD #1–#8 signed off; tag `v0.9.9-beta.N`**

## Phase 11 — Launch → `v1.0.0` (week 13)

- [ ] P11.1 Stable repos live (apt, dnf, AUR `-`/`-bin`)
- [ ] P11.2 Final QA vs PRD §11 checklist
- [ ] P11.3 README screenshots + landing copy
- [ ] P11.4 Tag `v1.0.0` + release notes
- [ ] **DoD: stable GA shipped; post-1.0 firewall roadmap opens**

---

## Release Gate Summary

| Gate | Requirement |
|---|---|
| `*-alpha.N` | phase DoD ticked; artifacts on GitHub Releases; breakage banner |
| `*-beta.N` | packaging matrix done (P8); no a11y blockers (P9); crash-free ≥ 99.5% |
| `v1.0.0` | PRD §11 DoD #1–#8 all signed; signed repos live |

**Flatpak:** not on the roadmap — flagged in PLAN.md §1 (beyond PRD scope; needs PRD update + user go-ahead). No checkbox until approved.
