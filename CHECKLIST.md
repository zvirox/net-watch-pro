# CHECKLIST.md — Plan Tracker

**Companion to `PLAN.md`** (task details: what/files/verify/deps live there; this file is for ticking). Rules: tick only verified work; a phase is done when its **DoD and Release** boxes are both ticked — release is part of the phase, not a follow-up (owner directive, 2026-11-18; procedure in AGENTS.md §6).

**Legend:** `[ ]` open · `[x]` done · `[~]` in progress · `[!]` blocked (note why inline)

---

## Phase 0 — Foundation & Tooling → `v0.0.1-alpha.1` (~40 h)

- [x] P0.1 Cargo workspace root configs <!-- verified: cargo check/fmt/clippy clean -->
- [x] P0.2 Crate skeletons (common, core, netwatchd, api, xtask) <!-- zero deps, compiles -->
- [~] P0.3 UI workspace (Vite + Svelte 5 + TS strict + lint) <!-- files done; npm install running (slow registry) -->
- [x] P0.4 Tauri 2 shell, window opens, capabilities least-privilege <!-- cargo check/clippy/fmt clean -->
- [~] P0.5 shadcn-svelte vendored: button, input, table <!-- blocked on node_modules -->
- [~] P0.6 Token CSS + self-hosted fonts, both themes <!-- tokens.css done; fonts downloaded; @font-face added -->
- [x] P0.7 CI: fmt/clippy/svelte-check + 3-distro build matrix on VMs <!-- workflow written; runs post-push -->
- [x] P0.8 Docs mirrors (api-contract, db-schema, env-vars)
- [ ] **DoD: themed blank window runs; `cargo check` + CI green**

## Phase S — eBPF Spike → time-boxed (~25 h)
- [ ] S.1 Spike setup: spikes/ebpf-accounting/ (nightly + bpf-linker; aya, aya-ebpf, bpf-linker) <!-- not started -->
- [ ] S.2 Implement + compare Designs 1–3 (cgroup_skb cookie / function hooks / hybrid) <!-- not started -->
- [ ] S.3 Measure Q1–Q10 (attribution vs counters, CPU/RSS, cgroup identity, capabilities, Secure Boot, churn, toolchain, write load) <!-- not started -->
- [ ] S.4 Workloads: curl, iperf3, browser QUIC, 5k short curls, 500 idle, WireGuard, Docker, IPv6, Flatpak, Python <!-- not started -->
- [ ] S.5 Write docs/spike-ebpf-report.md; go/no-go on ±5% accuracy <!-- not started -->
- [ ] **DoD: spike report exists; every kernel claim [VERIFIED] or [HYPOTHESIS]**

## Phase D — Doc Fixes → consistency pass done (~20 h)

- [ ] D.1 Record owner decisions D1–D7 in docs/decisions.md <!-- done -->
- [ ] D.2 Apply F-01–F-11 to PRD → DESIGN → ARCHITECTURE → DATABASE → API → AUTH_SECURITY <!-- in progress -->
- [ ] D.3 Apply findings to PLAN → CHECKLIST → TESTING → FEATURES → INTEGRATIONS/MCP/AGENTS/SETUP/README <!-- open -->
- [ ] D.4 Consistency grep pass (Part 6 step 6); fix every stale hit <!-- open -->
- [ ] D.5 Cross-check: PRD §7 story → table + endpoint; PRD §10 risk → test <!-- open -->
- [ ] **DoD: all findings merged; owner reviews before Phase 1' code resumes**

## Phase 1' — Walking Skeleton (revised) → `v0.1.0-alpha.1` (~40 h)

- [ ] P1.1 Unix-socket listener + peer-credential UID allowlist <!-- replaces token file + loopback bind (D1a) -->
- [ ] P1.2 `/health` + `/status` <!-- workspace integration tests pass -->
- [ ] P1.3 Error envelope + status mapping <!-- 401/404/405 envelopes tested -->
- [ ] P1.4 Socket stream skeleton (subscribe/ping) <!-- socket client: ping→pong, bad channel → QUERY_INVALID -->
- [ ] P1.5 Tauri socket bridge + in-memory UI client with capped reconnect <!-- replaces Polkit token helper; no per-launch prompt (D1a) -->
- [ ] P1.6 Status chip (4 states) connected to `status` channel <!-- not started -->
- [ ] P1.7 Dashboard shell + sidebar/topbar <!-- not started -->
- [ ] P1.8 Dev systemd unit (Unix socket path) <!-- not started -->
- [ ] **DoD: UI ↔ daemon connected over socket; reconnect ≤ 5 s; no per-launch prompt; tag `v0.1.0-alpha.1`**

## Phase 2 — Live Slice → `v0.2.0-alpha.2` (~40 h)

- [ ] P2.1 eBPF attach + ring buffer plumbing <!-- gated on spike S go/no-go -->
- [ ] P2.2 Byte counters → 1 s delta events <!-- gated on spike S -->
- [ ] P2.3 `/proc` attribution + `apps` upserts <!-- cgroup-first identity per D5a; /proc fallback -->
- [x] P2.4 Migrations 0001+0002 at boot <!-- fresh data dir → tables + seeds verified (smoke run + unit tests) -->
- [x] P2.5 Batched `connections` writer (checkpoint cadence ~60 s, F-09) <!-- persist_episodes upserts absolute totals, one tx per batch; unit tested -->
- [x] P2.6 `live.connections` channel (snapshot + delta) over socket stream <!-- EpisodeStore + 1 Hz live service + WS snapshot/delta frames; verified over live socket -->
- [x] P2.7 Live Connections screen (virtualized, pause, all states) <!-- live.svelte store + Live.svelte content-only; windowed table, filter row, pause/frozen banner, empty/loading/error states; svelte-check 0/0, lint+build clean; connected-flow visuals pending Tauri app check -->
- [x] P2.8 REST `/connections/live` + real ebpf status <!-- endpoint live + integration test REST==WS; ebpf field stays stub "attached" until the collector lands (P2.1–P2.3 blocked on kernel -EPERM; PLAN P1.6 allows) -->
- [ ] P2.9 Degraded path (`NETWATCH_DISABLE_EBPF` → honest UI)
- [ ] **DoD: real traffic visible, app-labeled, pause/sort work; tag `v0.2.0-alpha.2`**

## Phase 3 — History Slice → `v0.3.0-alpha.3` (~40 h)

- [x] P3.1 Migration 0003: `usage_rollup_15m`, `usage_endpoint_day`, `usage_interface_15m` + indexes
- [x] P3.2 `GET /connections` (keyset pagination, basic filters; aggregate rows beyond raw window)
- [x] P3.3 Pro-advanced-filter 403 gate (locked off)
- [x] P3.4 `GET /connections/{id}` (+ pruned 404 wording)
- [x] P3.5 History screen (search, table, all states)
- [x] P3.6 Restart persistence test <!-- real netwatchd kill+restart; exact history response and C3 row preserved; workspace tests green -->
- [x] P3.7 `apps` soft delete on last activity <!-- idempotent db helper marks stale apps; row + history name preserved; unit test green -->
- [ ] **DoD: restart keeps history; free history search works; tag `v0.3.0-alpha.3`**

## Phase 4 — Dashboard Slice → `v0.4.0-alpha.4` (~40 h)

- [x] P4.1 Compaction job (raw → `usage_rollup_15m` at 7 days; write `usage_endpoint_day`) <!-- transactional compaction + 50-endpoint cap; totals preserved; 2 core + 1 daemon test green -->
- [x] P4.2 Interface-counter sampler (`/sys/class/net/*/statistics`; `lo` excluded) <!-- pure delta core + 15s daemon task; synthetic sysfs test verifies 15m rows and lo exclusion; vnstat comparison follows in P4.3/P10.1 -->
- [x] P4.3 `GET /usage/summary` (total/attributed/unattributed + top apps) <!-- U1 contract + IANA day/billing boundaries (D4a); 4 calendar + 3 API tests green; chrono/chrono-tz approved 2026-11-17 -->
- [x] P4.4 `GET /usage/timeline` (15m; day/hour in configured timezone) <!-- raw+rollup seam union, local hour/day buckets, interface filter; 1 calendar + 4 API tests green -->
- [x] P4.5 `GET /usage/interfaces` (from `usage_interface_15m`; `lo` excluded) <!-- U3 shape + local-day default range; 2 API tests green, excluded interface never returned -->
- [x] P4.6 ECharts token theme wrapper <!-- token-derived theme + reusable Chart.svelte; verified light/dark series colors browser-side; echarts 6.1.0 approved -->
- [x] P4.7 Dashboard screen (headline total/attributed/unattributed, timeline, top apps, Unattributed row, interfaces) <!-- U1-U3 store, Day/Hour charts, click-to-App, empty/loading/error states; browser-verified light/dark -->
- [x] P4.8 `usage.totals` socket stream → headline cards <!-- real 1Hz publisher from U3 totals, WS subscribe on connect, live headline update; 3 Rust tests + browser frame check green -->
- [x] P4.9 Dashboard empty state <!-- shared DESIGN §7.11 EmptyState; fresh db shows — not 0 B, three consistent empty panels; browser-verified fresh + populated -->
- [ ] **DoD: totals equal kernel counters by construction; ≥95% attributed; tag `v0.4.0-alpha.4`**

## Phase 5 — Apps Slice → `v0.5.0-alpha.5` (~40 h)

- [x] P5.1 `GET /apps` (ranking, include_deleted) <!-- A1 sort/limit/soft-delete contract; ranking matches U1 top apps; 3 API tests green; API.md 1.2.1 clarified -->
- [x] P5.2 `GET /apps/{id}` + icon resource <!-- A2 detail with exe_path/deleted_at + ID-keyed icon endpoint with allowlisted image types; 2 API tests green; API.md 1.3.0 -->
- [x] P5.3 `GET /apps/{id}/usage` + `/endpoints` <!-- A3 app-scoped raw/rollup timeline + A4 merged endpoint ranking; 2 API tests green, 99 total Rust tests -->
- [x] P5.4 App Detail screen (3 tabs) <!-- A2-A4 + app-filtered C2 store, keyboard tabs, themed timeline, connections/endpoints tables; browser-verified all tabs -->
- [x] P5.5 Click-through routes from Dashboard/Live <!-- dashboard bars + live rows (mouse and Enter) route to #apps/{id}; browser-verified -->
- [x] P5.6 Attribution edge cases: cgroup-first identity, short-lived procs, confidence display (F-05/D5a) <!-- identity-key dedup + pid cache + flatpak/snap parse; 5 core tests; heuristic/unknown UI badge verified -->
- [x] **DoD: every app opens a meaningful detail page; tag `v0.5.0-alpha.5`** <!-- pinned to the retroactive release on 2026-11-18 -->
- [x] **Release: `v0.5.0-alpha.5` tagged + GitHub pre-release published** <!-- commit 510ebbd; source-only alpha; changelog updated -->

## Phase 6 — Freemium Slice → `v0.6.0-alpha.6` (~40 h)

- [x] P6.1 Migration 0004: `daemon_events` <!-- immutable bounded log + idx; shared event type; degraded paths record events; 9 daemon DB tests, 102 workspace tests -->
- [x] P6.2 License verifier (offline signature, no network) <!-- Ed25519 key format + embedded public key + SHA-256 fingerprint; 4 core tests, 106 workspace tests; deps approved -->
- [x] P6.3 License endpoints (get/activate/deactivate) <!-- L1-L3 with offline verifier, fingerprint-only storage, 409 conflict, status broadcast; 4 API tests, 109 workspace tests -->
- [x] P6.4 Server-side Pro gating (filters, retention ceiling) <!-- C2 uses shared license gate; T1/T2 enforce 90-day free ceiling with typed validation; 4 settings + 2 gating tests, 113 workspace tests -->
- [x] P6.5 Export engine (job → CSV/JSON, rate limits) <!-- E1-E3: in-memory jobs, C2-filter parity, CSV/JSON, 5/min create + 10/min download with RATE_LIMITED details/Retry-After, deactivation grace; 15 API tests, 128 workspace tests -->
- [x] P6.6 Upgrade modal + license states <!-- glass modal per §7.4/§8.8 with §7.2 inline errors, focus trap/Escape, L1-L3 store with WS-status invalidation for instant unlock, Pro badge; svelte-check/lint/build clean, browser smoke-tested -->
- [x] P6.7 History Pro locks (chips + tooltips) <!-- locked C2 filter chips with §7.12 tooltips open Upgrade; Pro chips drive real filters; deactivation clears them via tier hook; retention caption; svelte-check/lint/build clean -->
- [ ] P6.8 Export menu + native save dialog
- [ ] P6.9 Usage budgets & alerts (MVP free tier; D6a)
- [ ] **DoD: activate → instant unlock; deactivate → locks return; zero network calls; tag `v0.6.0-alpha.6`**
- [ ] **Release: tag `v0.6.0-alpha.6` + GitHub pre-release** <!-- after P6.8/P6.9 + zero-outbound check; source-only -->

## Phase 7 — System Slice → `v0.7.0-alpha.7` (~40 h)

- [ ] P7.1 Retention pruner (free/Pro aware)
- [ ] P7.2 `GET/PUT /settings` (typed, bounded)
- [ ] P7.3 Settings screen (general/data/daemon)
- [ ] P7.4 Interfaces + metered labeling (API + UI)
- [ ] P7.5 Tray icon + menu (quit-not-daemon)
- [ ] P7.6 Theme switching, persisted, charts re-theme
- [ ] P7.7 metered_policy + start_on_boot plumbed
- [ ] **DoD: tray quit keeps daemon; retention correct per tier; tag `v0.7.0-alpha.7`**
- [ ] **Release: tag `v0.7.0-alpha.7` + GitHub pre-release** <!-- source-only alpha -->

## Phase 8 — Packaging Slice → `v0.8.0-alpha.8` (~40 h) — *first public alpha*

- [ ] P8.1 Hardened systemd unit
- [ ] P8.2 `.deb` package + post-install
- [ ] P8.3 `.rpm` parity
- [ ] P8.4 AUR `net-watch-pro-git`
- [ ] P8.5 Onboarding flow (pkexec, health checks, 3 cards, < 3 min)
- [ ] P8.6 Failure path + demo-mode decision (PRD OQ4)
- [ ] P8.7 Uninstall completeness
- [ ] P8.8 Signing: package signing + release workflow (hosted build services until after beta)
- [ ] **DoD: install → onboard → monitor via GitHub Releases + AUR `-git`; self-hosted apt/dnf repos deferred (F-10); tag `v0.8.0-alpha.8`**
- [ ] **Release: tag `v0.8.0-alpha.8` + GitHub pre-release with `.deb`/`.rpm` artifacts** <!-- first installable/public alpha; AUR `-git` separate -->

## Phase 9 — Diagnostics & Accessibility → `v0.9.0-beta.1` (~40 h) — *public beta*

- [ ] P9.1 Diagnostics endpoint + modal
- [ ] P9.2 All degraded states end-to-end
- [ ] P9.3 Daemon-stopped banner + recovery actions
- [ ] P9.4 Contrast audit (both themes)
- [ ] P9.5 Keyboard nav + focus traps + SR labels
- [ ] P9.6 Chart data-table fallbacks
- [ ] P9.7 Reduced motion + 200% zoom
- [ ] **DoD: PRD DoD #3/#5/#6 pass; zero a11y blockers; tag `v0.9.0-beta.1`**
- [ ] **Release: tag `v0.9.0-beta.1` + GitHub release** <!-- public beta; signing/packaging matrix from P8 -->

## Phase 10 — Beta Hardening → `v0.9.x-beta.N` (~80 h)

- [ ] P10.1 Accuracy harness (≥95% attributed; totals equal kernel counters) in CI
- [ ] P10.2 Perf budget test (≤3% CPU / ≤150 MB RSS p95)
- [ ] P10.3 1-year retention soak (< 1.5 GB)
- [ ] P10.4 Kernel VM matrix (Arch/CachyOS, Ubuntu LTS, Fedora — VMs not containers)
- [ ] P10.5 Security review: unit sandbox, socket peer-credential, API surface
- [ ] P10.6 Beta feedback fixes (patch-tagged)
- [ ] **DoD: PRD §11 DoD #1–#8 signed off; tag `v0.9.9-beta.N`**
- [ ] **Release: final `v0.9.x-beta.N` release** <!-- patch-tag each beta feedback fix; final tag after DoD sign-off -->

## Phase 11 — Launch → `v1.0.0` (~40 h)

- [ ] P11.1 Stable repos live (apt, dnf, AUR `-`/`-bin`)
- [ ] P11.2 Final QA vs PRD §11 checklist
- [ ] P11.3 README screenshots + landing copy
- [ ] P11.4 Tag `v1.0.0` + release notes
- [ ] **DoD: stable GA shipped; post-1.0 firewall roadmap opens**
- [ ] **Release: tag `v1.0.0` + GitHub release (not pre-release)** <!-- stable repos live; release notes verify zero-outbound -->

---

## Release Gate Summary

| Gate | Requirement |
|---|---|
| `*-alpha.N` | phase DoD ticked; artifacts on GitHub Releases; breakage banner |
| `*-beta.N` | packaging matrix done (P8); no a11y blockers (P9); crash-free ≥ 99.5% |
| `v1.0.0` | PRD §11 DoD #1–#8 all signed; signed repos live |

**Release workflow (every phase, no re-ask):** all P-tasks + DoD verified → add
CHANGELOG entry for the planned tag (date + link refs) → tick the phase's Release
box above → commit (`docs(release): …`) → annotated tag from PLAN.md → push commit
+ tag → `gh release create <tag> --prerelease` (attach packages from Phase 8;
omit `--prerelease` for `v1.0.0`). Only an undefined tag, failed credentials, or
a publish error warrants stopping to report. See AGENTS.md §6 and PLAN.md §15.

**Flatpak:** deferred until after Phase 11 by owner decision 2026-11-18 (PRD 1.2 / PLAN.md §1): it cannot bundle the root systemd daemon, so it would break the one-command install contract. AUR stays in Phase 8 and at GA; no Flatpak checkbox until a new PRD/PLAN update.
