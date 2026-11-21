# PLAN.md — Implementation Roadmap

**Version:** 1.3 · **Date:** 2026-11-19
**Sources:** `PRD.md` §6 (features), `ARCHITECTURE.md` §4 (structure), `DATABASE.md` §8 (migrations), `API.md` §2 (endpoints) · `docs/decisions.md` (owner decisions D1–D7)

**Companion tracker:** `CHECKLIST.md` (same task IDs, ticked live)

**Version history:** 1.2 (2026-11-15): apply plan-fix findings — insert Phase S (eBPF spike) and Phase D (doc fixes); reorder phases per Part 5; replace week targets with effort estimates (D7: 40+ h/week); rewrite P1.1–P1.5 for Unix-socket + peer-credential transport (D1a); defer self-hosted apt/dnf repos until after beta (F-10). 1.3 (2026-11-19): add Phase L (Live Throughput, F11) with tasks A1–A8, owner-run spike B, collector C and range presets D; full detail in `docs/LIVE-THROUGHPUT-PLAN.md`.

> **Ordering rules:** vertical slices only — every phase ends with a runnable app; never all-backend-then-all-frontend. Each task ≤ 2 hours. Dependencies use task IDs (e.g. `dep: P1.3`). A task may start when its deps are done.

---

## 1. Phase Overview & Release Timeline

| Phase | Name | Output (runnable state after phase) | Tag at phase end | Est. hours |
|---|---|---|---|---|
| 0 | Foundation & tooling | Repo scaffolded, CI green, empty Tauri window runs | `v0.0.1-alpha.1` | ~40 h |
| **S** | **eBPF spike (new)** | `spikes/ebpf-accounting/` + `docs/spike-ebpf-report.md`; gates everything below | — | time-boxed (~25 h) |
| **D** | **Doc fixes (new)** | All Part 4 edits merged, consistency pass done | — | ~20 h |
| 1' | Walking skeleton (revised) | Daemon + UI talk over Unix socket + peer-credential; status chip live | `v0.1.0-alpha.1` | ~40 h |
| 2' | Live slice | Real data from spike-chosen collector on the Live screen; accuracy harness running | `v0.2.0-alpha.2` | ~40 h |
| 3' | History slice | Persistence with episode definition (F-09), raw + endpoint rollups (F-02) | `v0.3.0-alpha.3` | ~40 h |
| 4' | Dashboard slice | Totals from interface counters, Unattributed row, timezone/billing-cycle correct (F-03/F-04) | `v0.4.0-alpha.4` | ~40 h |
| 5 | Apps slice | Cgroup-based identity (F-05), App detail | `v0.5.0-alpha.5` | ~40 h |
| 6 | Freemium slice (reduced) | License verify + simple gating (F-07) | `v0.6.0-alpha.6` | ~40 h |
| 7 | System slice | Tray, settings, retention/pruning | `v0.7.0-alpha.7` | ~40 h |
| 8' | Packaging (reduced) | GitHub Releases `.deb`/`.rpm`, AUR `-git`; hosted repos deferred (F-10) | `v0.8.0-alpha.8` | ~40 h |
| 9 | Diagnostics & a11y | All degraded states + accessibility pass | `v0.9.0-beta.1` | ~40 h |
| 10' | Hardening | Kernel VM matrix, perf budgets, external review (F-08) | `v0.9.x-beta.N` | ~80 h |
| 11 | Launch | Stable 1.0 GA + repos live | `v1.0.0` | ~40 h |

**Tag format:** `vMAJOR.MINOR.PATCH-alpha.N` / `-beta.N` during pre-releases; plain `v1.0.0` at GA. Within a phase, patch bumps (`v0.3.1-alpha.3`) mark internal fixes worth tagging. Feature phases are MINOR bumps; the phase *number maps loosely* — the alpha/beta suffix is the real maturity signal.

### Packaging matrix (what ships at each release stage)

| Stage | Artifacts | Channels |
|---|---|---|
| Alpha (`v0.*.0-alpha.N`) | GitHub tarballs + `.deb` + `.rpm` (unpolished) | GitHub Releases only, "expect breakage" banner |
| Beta (`v0.9.0-beta.N`) | Signed `.deb` + apt repo, `.rpm` + dnf repo, AUR `net-watch-pro-git` (dev channel) | GitHub Releases + beta repo subdomain; testers wanted |
| Stable (`v1.0.0`) | Signed `.deb` (apt repo), `.rpm` (dnf repo), AUR `net-watch-pro` + `net-watch-pro-bin` | PRD §6.1 #10 complete |

**Packaging deferral (F-10):** self-hosted apt/dnf repos and signing keys are deferred until after beta. Alpha/beta ship GitHub Releases artifacts + AUR `-git`; prefer hosted build services (COPR, PPA, openSUSE Build Service) over running repo infrastructure alone. External security review before 1.0, not in the first-alpha path.

**Flatpak:** decision (owner, 2026-11-18; PRD 1.2) — **deferred until after Phase 11 (v1.0.0)**. A sandboxed Flatpak cannot host the root systemd daemon; it would be a UI-only wrapper plus a separate host install, breaking the one-command install promise. **AUR stays the Arch/CachyOS install path at Phase 8** (`net-watch-pro-git`, then `-`/`-bin` at GA). Any Flatpak work needs a new PRD/PLAN update and owner go-ahead.

---

## 2. Phase 0 — Foundation & Tooling

**Goal:** workspace exists, checks run in CI, an empty Tauri window opens.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P0.1 | Cargo workspace: root `Cargo.toml` (members), `rust-toolchain.toml`, `deny.toml`, `clippy.toml` | root configs | `cargo check` passes on empty workspace | — |
| P0.2 | Create crates as empty skeletons: `netwatch-common`, `netwatch-core`, `netwatchd`, `netwatch-api` (+ `xtask`) | `crates/*/` | `cargo build` compiles all members | P0.1 |
| P0.3 | UI workspace: `app/` with Vite + Svelte 5 + TS strict + ESLint/Prettier; empty `App.svelte` | `app/*` | `npm run dev` serves; `svelte-check` clean | — |
| P0.4 | Tauri 2 shell added to `app/src-tauri/`, window opens; `capabilities/` least-privilege | `app/src-tauri/*` | `tauri dev` opens blank window (DESIGN.md bg) | P0.3 |
| P0.5 | Vendored shadcn-svelte primitives: button, input, table only | `app/src/lib/components/ui/` | storyless smoke: button renders in App.svelte | P0.4 |
| P0.6 | Token CSS layer: DESIGN.md §3–§6 tokens as CSS vars + fonts (self-hosted Inter/JetBrains Mono) | `app/src/lib/theme/` | window shows both themes on toggle | P0.5 |
| P0.7 | CI: fmt, clippy `-D warnings`, svelte-check, build matrix (arch/deb/rpm images) | `.github/workflows/ci.yml` | CI green on empty repo | P0.2, P0.3 |
| P0.8 | Docs mirror: `docs/api-contract.md`, `docs/db-schema.md`, `docs/env-vars.md` generated from API.md/DATABASE.md | `docs/*` | mirrors match; drift check stub | — |

**DoD:** `tauri dev` shows a themed blank window; `cargo check` + CI green; AGENTS.md checklist passes for all files.

---

## S. Phase S — eBPF Spike (new)

**Goal:** replace guesses about eBPF accounting with measurements before any production collector code is written. Time box: ~25 hours (1–2 weeks). Stop at the box and report even if unfinished.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| S.1 | Spike setup: `spikes/ebpf-accounting/` with own `Cargo.toml`/`rust-toolchain.toml` (nightly + bpf-linker); deps: `aya`, `aya-ebpf`, `bpf-linker`; dev tools: `iperf3`, `bpftool` | `spikes/ebpf-accounting/` | spike crate builds on nightly | P0.2 |
| S.2 | Implement + compare Designs 1–3 (cgroup_skb cookie / function hooks / hybrid) on workloads (S.4) | `spikes/ebpf-accounting/src/` | decision table: accuracy, overhead, kernel floor, complexity, failure modes | S.1 |
| S.3 | Measure Q1–Q10: hook attribution vs interface counters, CPU/RSS, cgroup identity, capabilities, Secure Boot/lockdown, churn, toolchain reality, write load | `spikes/ebpf-accounting/`, `tests/` fixtures | measurement tables; each Q answered with data | S.2 |
| S.4 | Workloads: curl down/up, iperf3 TCP/UDP, browser QUIC/HTTP3, 5k short curls, 500 idle conns, WireGuard, Docker, IPv6-only, Flatpak, Python script | `spikes/ebpf-accounting/` | per-workload: interface delta, attributed total, per-app split, unattributed remainder | S.2 |
| S.5 | Write `docs/spike-ebpf-report.md`: Q1–Q10 answers, chosen design, rejected designs with reasons, product claims to soften/drop | `docs/spike-ebpf-report.md` | report reviewed by owner | S.3 |

**DoD:** spike report exists; every kernel/eBPF claim in docs tagged `[VERIFIED: spike-ebpf-report §n]` or explicitly `[HYPOTHESIS]`; go/no-go on per-app ±5% accuracy.

---

## D. Phase D — Doc Fixes (new)

**Goal:** apply all Part 4 findings (F-01–F-11) to planning docs with version bumps + consistency pass.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| D.1 | Record owner decisions D1–D7 in `docs/decisions.md` | `docs/decisions.md` | decisions file complete, dated | S (informational) |
| D.2 | Apply F-01–F-11 to `PRD.md` → `DESIGN.md` → `ARCHITECTURE.md` → `DATABASE.md` → `API.md` → `AUTH_SECURITY.md` | all source-of-truth docs | each doc version bumped + rationale line | D.1 |
| D.3 | Apply findings to `PLAN.md` → `CHECKLIST.md` → `TESTING.md` → `FEATURES.md` → `INTEGRATIONS.md`/`MCP.md`/`AGENTS.md`/`SETUP.md`/`README.md` | remaining docs | consistency; no two docs contradict on retention/accuracy/transport/capabilities/timezone | D.2 |
| D.4 | Consistency grep pass (Part 6 step 6) for stale terms; fix every hit | all docs | grep returns no stale hits | D.3 |
| D.5 | Cross-check: every PRD §7 story → table + endpoint; every PRD §10 risk → test | all docs | matrix complete | D.4 |

**DoD:** all findings merged; consistency pass clean; owner reviews before Phase 1' code resumes.

---

## 3. Phase 1' — Walking Skeleton (revised)

**Goal:** daemon and UI talk to each other over a Unix socket with peer-credential authorization. No traffic data yet.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P1.1 | Unix-socket listener + peer-credential UID allowlist; allowed-UID list written during onboarding pkexec | `crates/netwatchd/src/{config,api}.rs`, `packaging/systemd/netwatchd.service` | daemon creates socket (0660, dir 0750), curl without allowed UID → connection refused; allowed UID → 200 | P0.2 |
| P1.2 | `GET /health` (no auth) + `GET /status` per API.md §2.1 | `crates/netwatch-api/src/` | curl both endpoints returns §2.1 payloads | P1.1 |
| P1.3 | Error envelope + status-code mapping as shared helpers | `crates/netwatch-api/src/` | error body matches API.md §1.3 exactly | P1.2 |
| P1.4 | Socket stream skeleton: `subscribe`/`unsubscribe`/`ping` | `crates/netwatch-api/src/` | socket client: ping→pong, bad channel → `QUERY_INVALID` frame | P1.3 |
| P1.5 | Tauri retrieval command + authenticated native socket bridge; in-memory UI REST/socket client with capped reconnect | `crates/netwatch-api/src/auth.rs`, `app/src-tauri/`, `app/src/lib/api/` | API rejects UID not in allowlist and accepts allowed UID; socket stream auth works; daemon restart rotates nothing (UID list persists) and UI recovers ≤ 5 s; no token persistence/logging | P1.4 |
| P1.6 | Status chip component (4 states) wired to `status` channel | `app/src/lib/components/`, `app/src/lib/screens/dashboard/` | flip daemon states manually → chip follows (DESIGN.md §7.5) | P1.5, P0.6 |
| P1.7 | Empty Dashboard screen shell + sidebar/topbar shell (DESIGN.md §8.1) | `app/src/screens/` | routes render; compact/wide reflow sanity | P0.5 |
| P1.8 | Dev systemd unit (Unix socket path, no hardening yet) | `packaging/systemd/netwatchd.service` | `systemctl start netwatchd` → UI connects over socket | P1.2 |

**DoD:** start daemon + `tauri dev` → chip shows "Monitoring" (stub state), reconnect works, CI green, socket + peer-credential verified on Arch/CachyOS. No per-launch admin prompt (D1a). No production-readiness claim before runtime + packaging checks pass.

---

## 4. Phase 2 — Live Slice (first real data)

**Goal:** the Live Connections screen shows real traffic. eBPF observe-only.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P2.1 | eBPF skeleton: attach empty socket trace, ring buffer plumbed | `crates/netwatch-ebpf/`, `crates/netwatchd/src/ebpf_loader.rs` | daemon logs attach OK on test VM | P1.1 |
| P2.2 | Per-socket `FLOWS` map (cookie + IPv4 tuple + absolute counters) → 1 Hz `EpisodeEvent`s | `crates/netwatch-ebpf/src/main.rs`, `crates/netwatchd/src/ebpf.rs` | counters move while downloading a file | P2.1 |
| P2.3 | cgroup→app registry resolution for episode rows; PID best-effort from scope name (0 = unknown) | `crates/netwatchd/src/db.rs`, `netwatch-common/src/app_names.rs` | new connection → row in `apps` + `connections` with confidence | P2.2 |
| P2.4 | Migration 0001+0002 applied at daemon boot (sqlx) | `crates/netwatchd/src/db/migrations/` | fresh data dir → tables + seeds exist (DATABASE.md §8) | P1.1 |
| P2.5 | `connections` batch writer (≤ 1 tx/s equivalent from in-memory deltas, checkpoint cadence ~60 s) | `crates/netwatchd/src/db.rs` | sqlite row counts grow during traffic; tx rate ≤ 1/s | P2.3, P2.4 |
| P2.6 | `live.connections` channel: snapshot + deltas ≤1 msg/s over socket stream | `crates/netwatch-api/src/` | socket client sees snapshot then deltas | P2.5, P1.4 |
| P2.7 | Live Connections screen: virtualized table per DESIGN.md §8.4 | `app/src/lib/screens/live/` | browse → rows stream; pause freezes; empty/loading/error states present | P2.6, P0.5 |
| P2.8 | `GET /connections/live` + `GET /status` show real ebpf state | `crates/netwatch-api/src/` | REST snapshot matches WS snapshot | P2.6 |
| P2.9 | Degraded path: `NETWATCH_DISABLE_EBPF=1` → warning chip + banner | daemon + `app/src/lib/stores/` | set env → UI shows Degraded honestly | P2.1, P1.6 |

**DoD:** with real browsing traffic, Live screen shows correct app-labeled rows; pause/sort work; accuracy eyeball ≈ interface counters; tag `v0.2.0-alpha.2`.

---

## 5. Phase 3 — History Slice

**Goal:** data survives restarts; History screen works (free tier).

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P3.1 | Migration 0003: `usage_rollup_15m`, `usage_endpoint_day`, `usage_interface_15m` + indexes | `crates/netwatchd/src/db/migrations/` | apply on existing db; tables + indexes exist (DATABASE.md §8) | P2.4 |
| P3.2 | `GET /connections` (keyset pagination, basic filters; aggregate rows beyond raw window) | `crates/netwatch-api/src/` | curl pagination walk returns stable order; aggregate flag works | P2.5, P1.3 |
| P3.3 | Pro-advanced-filter gate (403 path) scaffolded but locked off | `crates/netwatch-api/src/` | advanced filter now → `EXPORT_PRO_REQUIRED` | P3.2 |
| P3.4 | `GET /connections/{id}` incl. pruned-window 404 wording | `crates/netwatch-api/src/` | fake id → 404 message per API.md §2.2 | P3.2 |
| P3.5 | History screen: search by app, table, empty/loading/error (§8.6) | `app/src/lib/screens/history/` | browse real history; pagination infinite-scroll | P3.2, P0.5 |
| P3.6 | Restart persistence test: daemon restart keeps history | `tests/integration/` | kill+restart → rows still queryable | P3.2 |
| P3.7 | Soft delete behavior: `apps.deleted_at` on last activity | `netwatch-core/src/attribution.rs` | stale app prunes → history still resolves names | P2.3, P3.1 |

**DoD:** full restart loses nothing; History answers "what used data yesterday"; tag `v0.3.0-alpha.3`.

---

## 6. Phase 4 — Dashboard Slice

**Goal:** the landing page shows the real numbers.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P4.1 | Compaction job: raw > 7 days → `usage_rollup_15m`; write `usage_endpoint_day` (capped N) | `netwatch-core/src/retention.rs` | seed old rows → rollups appear, raw pruned | P3.1 |
| P4.2 | Interface-counter sampler task (reads `/sys/class/net/*/statistics`; `lo` excluded) | `netwatch-core/src/` | sampler writes `usage_interface_15m`; headline matches `vnstat` | P3.1 |
| P4.3 | `GET /usage/summary` (total/attributed/unattributed + top apps) | `crates/netwatch-api/src/` | totals equal kernel counters by construction | P4.1, P4.2 |
| P4.4 | `GET /usage/timeline` (15m; day/hour display in configured timezone) | `crates/netwatch-api/src/` | series continuity across 7-day seam (flag F-05) | P4.1 |
| P4.5 | `GET /usage/interfaces` (from `usage_interface_15m`, `lo` excluded) | `crates/netwatch-api/src/` | excluded `lo` never counted | P4.2 |
| P4.6 | ECharts theme wrapper from DESIGN.md §3.3 | `app/src/lib/components/charts/` | series order/colors match tokens; theme switch works | P0.6 |
| P4.7 | Dashboard screen: headline cards (total/attributed/unattributed) + timeline + top apps + Unattributed row + interfaces (§8.3) | `app/src/lib/screens/dashboard/` | chart → click bar → App detail stub routes | P4.2–P4.6 |
| P4.8 | `usage.totals` socket stream wired to headline cards | `crates/netwatch-api/src/`, stores | totals tick without polling | P4.3, P1.5 |
| P4.9 | Dashboard empty state ("Waiting for first data") | dashboard screen | fresh db → empty state, not zeros lie | P4.7 |

**DoD:** dashboard totals equal kernel interface counters by construction; ≥95% attributed; Unattributed row shows gap; charts themed; tag `v0.4.0-alpha.4`.

---

## 7. Phase 5 — Apps Slice

**Goal:** click-through from anywhere to a working App detail page.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P5.1 | `GET /apps` (ranking, include_deleted) | `crates/netwatch-api/src/` | top apps sort matches summary | P4.2 |
| P5.2 | `GET /apps/{id}` + icon resource endpoint | `crates/netwatch-api/src/` | unknown icon → UI fallback (DESIGN.md §6.2) | P5.1 |
| P5.3 | `GET /apps/{id}/usage` + `/endpoints` (from `usage_endpoint_day`) | `crates/netwatch-api/src/` | endpoint ranking matches manual query | P5.1 |
| P5.4 | App Detail screen: tabs, usage chart, connections table, endpoint list (§8.5) | `app/src/lib/screens/app-detail/` | all three tabs populated from real data | P5.2, P5.3, P2.6 |
| P5.5 | Route from Dashboard top-apps + Live rows to detail | dashboard/live screens | click-through lands with correct id | P5.4 |
| P5.6 | Attribution edge cases: cgroup-first identity, short-lived procs, confidence display | `netwatch-core/src/attribution.rs` | unknown procs show `heuristic/unknown` badge honestly (F-05/D5a) | P2.3 |

**DoD:** every visible app opens a meaningful detail page; no dead-end routes; tag `v0.5.0-alpha.5`.

---

## 8. Phase 6 — Freemium Slice

**Goal:** the free/Pro boundary is real, server-side, and offline.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P6.1 | Migration 0004: `daemon_events` | `crates/netwatchd/src/db/migrations/` | events land on attach failure | P3.1 |
| P6.2 | License table + verifier: signature check, no network | `crates/netwatch-core/src/license.rs`, `packaging/licenses/` | valid key → pro; tampered → `LICENSE_INVALID` | P2.4 |
| P6.3 | `GET /license` + `POST /license/activate` + `DELETE /license` | `crates/netwatch-api/src/` | activate/deactivate flows return L1 shape (API.md §2.8) | P6.2 |
| P6.4 | Server-side gating wired: C2 advanced filters, T2 retention ceiling | `crates/netwatch-api/src/` | free token: advanced filters → 403; pro → 200 | P6.3, P3.3 |
| P6.5 | Export engine: `POST /exports` → job → content (CSV+JSON) | `crates/netwatch-api/src/`, `netwatchd/src/` | export downloads; rate limit 5/min hits 429 | P6.4 |
| P6.6 | Upgrade modal (§8.8) + license entry screen states | `app/src/lib/screens/upgrade/` | invalid key → inline error (§7.2); success unlocks instantly via WS `status` | P6.3, P0.5 |
| P6.7 | History screen Pro locks: disabled filter chips + tooltips | `app/src/lib/screens/history/` | free tier sees locks; upgrade modal opens | P6.6, P3.5 |
| P6.8 | Export menu + native save dialog in History | `app/src/lib/screens/history/` | CSV/JSON file lands via dialog | P6.5, P6.7 |
| P6.9 | Usage budgets & alerts (MVP free tier): per-app + total daily/monthly budgets, desktop notifications | `crates/netwatch-core/src/`, `app/src/lib/screens/settings/` | budget breach → notification; metered-interface aware (D6a) | P7.2 |

**DoD:** full freemium loop: activate → features unlock without restart; deactivate → locks return; no network calls at any point; tag `v0.6.0-alpha.6`.

---

## 9. Phase 7 — System Slice (tray, settings, retention)

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P7.1 | Retention pruner (90-day default, Pro ceiling from license) | `netwatch-core/src/retention.rs` | backdated rows vanish; rollups survive | P4.1, P6.2 |
| P7.2 | `GET/PUT /settings` with type/bounds validation | `crates/netwatch-api/src/` | bad type/range → 400; free retention > 90 → 403 | P6.4 |
| P7.3 | Settings screen: general/data/daemon sections (§8.7) | `app/src/lib/screens/settings/` | retention change reflects in Diagnostics | P7.2 |
| P7.4 | Interfaces table + metered labeling (I1/I2 + UI) | `netwatchd/src/db.rs`, API, screens/settings | toggle metered → dashboard card updates | P4.4 |
| P7.5 | Tray icon + menu: today usage tooltip, open, quit-not-daemon | `app/src-tauri/src/tray.rs` | tray quit keeps daemon running (PRD Flow 6) | P1.8 |
| P7.6 | Theme switching (light/dark/system) persisted | `app/src/lib/stores/settings-ui.svelte.ts` | theme survives restart; ECharts re-theme | P4.5 |
| P7.7 | Metered-policy + start_on_boot settings plumbed | settings screens + daemon | behavior changes match settings | P7.2, P7.4 |

**DoD:** tray-quit test passes; retention math correct for free/pro; settings persist; tag `v0.7.0-alpha.7`.

---

## 10. Phase 8 — Packaging Slice (first installable)

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P8.1 | Hardened systemd unit: sandboxing + capability bounding (PRD §5) | `packaging/systemd/netwatchd.service` | `systemd-analyze security` score acceptable | P1.8 |
| P8.2 | `.deb` package + post-install (register unit, disabled-by-default) | `packaging/deb/` | clean VM install → menu entry, unit present | P8.1 |
| P8.3 | `.rpm` package parity | `packaging/rpm/` | clean VM install on Fedora | P8.2 |
| P8.4 | AUR `net-watch-pro-git` PKGBUILD (dev channel) | `packaging/arch/` | `paru -S net-watch-pro-git` builds | P8.2 |
| P8.5 | Onboarding flow: pkexec enable, health checks, 3 cards (§8.2) | `app/src/lib/screens/onboarding/` | fresh install → enabled in < 3 min | P8.2, P1.6 |
| P8.6 | Failure path: actionable error card + retry + demo-mode stub decision (OQ4) | onboarding screen | blocked kernel → guidance, not a dead end | P8.5 |
| P8.7 | Uninstall completeness: unit + data opt-in (PRD metric) | `packaging/*` | remove package → no service residue | P8.2 |
| P8.8 | Signing: package signing + release workflow (hosted build services preferred over self-hosted repos until after beta) | `.github/workflows/release.yml` | artifacts signed; clean-VM install works | P8.2, P8.3 |
| P8.9 | Embed the BPF object in `netwatchd` via `aya::include_bytes_aligned!`; keep `NETWATCH_EBPF_OBJECT` as a dev override only (owner-approved 2026-11-20) | `crates/netwatchd/build.rs`, `ebpf.rs`, `xtask` | no runtime object file needed; dev override still works | P8.1 |
| P8.10 | Build the object in CI and the AUR `PKGBUILD` (stable + `RUSTC_BOOTSTRAP=1` + `rust-src` + pinned `bpf-linker` 0.13.0); no `.o` in git | `release.yml`, `ci.yml`, `packaging/arch/` | binaries carry the object from a clean build | P8.9 |
| P8.11 | Clean-install test (fresh `makepkg -si` → one pkexec → `/status` `ebpf=attached`) and fix stale `.SRCINFO`/`pkgrel` | `packaging/tests/`, `packaging/arch/` | `[VERIFIED]` only from a real host run | P8.10 |
| P8.12 | Restart `netwatchd` on package upgrade so a new binary replaces the running one; never enable a stopped unit (owner-approved 2026-11-20) | `packaging/{arch,deb,rpm}` | upgrade test: restart only when the unit was active | P8.11 |

**DoD:** install → onboard → monitor via GitHub Releases artifacts + AUR `-git`; self-hosted apt/dnf repos deferred until after beta (F-10); tag `v0.8.0-alpha.8` = first public alpha. P8.9–P8.11 are post-release follow-ups approved 2026-11-20, not part of that tag.

---

## 11. Phase 9 — Diagnostics & Accessibility (beta gate)

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P9.1 | `GET /diagnostics` full payload + Diagnostics modal (§8.9) | API + `app/src/lib/screens/` | modal shows checks/journal/copy-report | P6.1 |
| P9.2 | All degraded states tested end-to-end (no daemon, no eBPF, perms) | daemon + UI | each PRD DoD #5 path exercised | P9.1, P2.9 |
| P9.3 | Daemon-stopped banner + Start-daemon pkexec action | UI | banner actions recover to Monitoring | P9.2 |
| P9.4 | A11y pass: contrast audit vs DESIGN.md §9.1 table | theme + components | automated contrast check green both themes | P0.6 |
| P9.5 | A11y pass: keyboard nav + focus traps + SR labels (§9.3–9.5) | all screens | screen-reader walkthrough script passes | P9.4 |
| P9.6 | Chart data-table fallbacks (§9.4) | chart wrappers | every chart has table equivalent | P4.5 |
| P9.7 | Reduced-motion + zoom-200% checks | theme | prefers-reduced-motion honored | P9.5 |

**DoD:** PRD DoD #3/#5/#6 all pass; a11y blockers at zero; tag `v0.9.0-beta.1` = public beta.

---

## 12. Phase 10 — Beta Hardening

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P10.1 | Accuracy harness: daemon vs interface counters (≥95% attributed, totals equal kernel counters) | `tests/accuracy/` | CI job green over synthetic 50-conn load; moved into spike then Phase 2 | P4.2 |
| P10.2 | Perf budget test: ≤3% CPU / ≤150 MB RSS p95 | `tests/integration/` | budget assertions in CI | P10.1 |
| P10.3 | 1-year retention soak: size < 1.5 GB | `tests/integration/` | seeded volume profile matches ARCHITECTURE.md §9 | P7.1 |
| P10.4 | Kernel VM matrix runs full test suite (Arch/CachyOS, Ubuntu LTS, Fedora — **VMs, not containers**) | CI | VM jobs green on all 3 families | P8.2, P8.3 |
| P10.5 | Security review: unit sandbox, socket peer-credential, API surface | daemon | review findings fixed/closed | P8.1 |
| P10.6 | Beta feedback fixes (buffer; scope-tight only) | as needed | each fix tagged patch `v0.9.x-beta.N` | P9 |

**DoD:** PRD §11 Definition of Done items #1–#8 all satisfied; crash-free ≥ 99.5% in beta; tag `v0.9.9-beta.N`.

---

## 13. Phase 11 — Launch (v1.0.0)

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P11.1 | Stable repos live: apt + dnf via hosted build services (COPR/PPA), AUR `net-watch-pro`/`-bin` | `packaging/*`, workflows | clean-VM repo installs on all 3 | P10.4 |
| P11.2 | Final QA sweep against PRD §11 checklist | — | every DoD item signed off | P10.6 |
| P11.3 | README screenshots + docs site/landing copy | `README.md`, `docs/` | no placeholders left | P9 |
| P11.4 | Tag `v1.0.0` + release notes (zero-outbound claim verifiable) | release | GA artifacts published | P11.1–P11.3 |

**DoD:** stable 1.0 shipped per PRD; post-1.0 firewall phase-in (PRD §12) planning opens with its own PLAN update.

---

## 14. Phase 12 — Starship Control & `starship-plugin` SDK Integration

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| P12.1 | Embed `starship-plugin` crate & publish manifest | `crates/netwatchd/`, `starship-plugin` | Manifest at `$XDG_DATA_HOME/starship/plugins/nwp.json` | P8.1 |
| P12.2 | Unix socket bridge listener with `SO_PEERCRED` | `crates/netwatchd/src/bridge.rs` | Socket at `$XDG_RUNTIME_DIR/starship/bridges/nwp.sock` verified | P12.1 |
| P12.3 | Expose MCP telemetry tools to Starship Control (`network.query_top_talkers`, `network.query_app_usage`, `network.audit_connections`) | `crates/netwatchd/src/mcp.rs` | Starship Control queries network state read-only | P12.2 |

**DoD:** Net Watch Pro is auto-discovered by Starship Control; autonomous agents can audit network leaks and per-app usage over the local Unix domain socket bridge.

---

## 15. Phase L — Live Throughput (F11)

**Full contract:** `docs/LIVE-THROUGHPUT-PLAN.md` (owner-approved scope change). Phase A is agent work; stop for approval afterwards. Phase B is owner-run on real hardware; Phase C starts only after the accepted spike report; Phase D can run in parallel with B.

| ID | Task | Files | Verify | Deps |
|---|---|---|---|---|
| L-A1 | Docs first, with version bumps | PRD, API, ARCHITECTURE, api-contract, env-vars, DATABASE, DESIGN, FEATURES, TESTING, PLAN, CHECKLIST | Contract docs merged before code | — |
| L-A2 | Pure rate math + tests | `netwatch-core/src/rates.rs` | counter reset/decrease, churn, late tick, smoothing | L-A1 |
| L-A3 | 1 Hz sysfs rate task + degraded path | `netwatchd/src/rates.rs`, `main.rs` | survives read errors, records degraded event | L-A2 |
| L-A4 | `net.rates` stream + shared payload | `netwatch-common`, `netwatch-api/src/stream.rs` | snapshot on subscribe, 1 Hz, unsubscribed gets none | L-A3 |
| L-A5 | Network now card + rates store | `app/src/lib/stores/`, `Dashboard.svelte` | loading/idle/stale/error states | L-A4 |
| L-A6 | `speed_unit` setting + migration 0005 | `DATABASE.md`, migration, `settings.rs`, settings screen | toggle applies without restart | L-A1 |
| L-A7 | Honest `/status` + chip | `netwatchd/src/main.rs`, `netwatch-common`, `StatusChip.svelte` | no collector → not green; chip explains why | L-A1 |
| L-A8 | Correct stale tracker claims | `CHECKLIST.md`, `PLAN.md` | P2.3 unticked, 1 Hz note fixed | L-A3 |
| L-OQ1 | 60-sample one-minute in-memory history + U4 endpoint | `netwatchd/src/rates.rs`, `netwatch-api` | 60 one-minute samples (last hour), `in_memory:true`, lost-on-restart label | L-A3 |
| L-OQ4 | Wi-Fi signal/link from local sysfs on U3 | `netwatch-api/src/interfaces_usage.rs`, Dashboard, TS types | injectable-root tests; "—" when missing | L-A1 |
| L-B1 | eBPF spike in Aya, Motrix workloads | `spikes/ebpf-aya/`, `docs/spike-ebpf-report.md` | scaffold builds + env diagnostics verified 2026-11-19; owner-run with real root still pending; `[VERIFIED]`/`[REFUTED]` claims | L-A |
| L-C1–C9 | Collector, identity, display names, `live.apps`, per-app speeds, sum check | `netwatch-ebpf/`, daemon, Apps/Live screens | accepted spike report | L-B1 |
| L-D1–D5 | Range presets 1 h/24 h/7 d/30 d/cycle, remembered via `range_default`; adds the Apps list screen and a forward-only settings migration (owner approved 2026-11-20) | usage/apps APIs, Dashboard, new `screens/apps/`, `stores/range.svelte.ts`, migration 0006 | timezone and cycle tests (F12) | L-A1 |

**DoD (Phase A):** live PC speed matches a known rate within ~10%, idle ~0, stale/no-collector states honest, zero outbound, unit + contract tests green. **Stop for owner approval before L-B/L-C.**

---

## 16. Global Rules

- **Runnable after every phase** — a phase that can't demo its slice isn't done.
- **Released after every phase** (owner directive, 2026-11-18) — the tag named in §1 / the phase DoD is part of the DoD, not a follow-up. Code complete with no tag and no GitHub release means the phase is still open. Release procedure: AGENTS.md §6.
- **Docs move with code** (AGENTS.md §4.5): the task that changes an endpoint/schema/token carries its doc edit.
- **No new dependencies** without explicit user approval (AGENTS.md §5).
- **OQ4 (demo mode)** is deliberately deferred to P8.6 — decide there, don't pre-build it.
- Timeline assumes a single full-time builder (40+ h/week, D7); estimates are in hours, labeled estimates; parallelize only P0 UI/daemon halves.
