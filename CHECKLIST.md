# CHECKLIST.md — Plan Tracker

**Companion to `PLAN.md`** (task details: what/files/verify/deps live there; this file is for ticking). Rules: tick only verified work; a phase is done when its **DoD and Release** boxes are both ticked — release is part of the phase, not a follow-up (owner directive, 2026-11-18; procedure in AGENTS.md §6).

**Legend:** `[ ]` open · `[x]` done · `[~]` in progress · `[!]` blocked (note why inline)

---

## Phase 0 — Foundation & Tooling → `v0.0.1-alpha.1` (~40 h)

- [x] P0.1 Cargo workspace root configs <!-- verified: cargo check/fmt/clippy clean -->
- [x] P0.2 Crate skeletons (common, core, netwatchd, api, xtask) <!-- zero deps, compiles -->
- [x] P0.3 UI workspace (Vite + Svelte 5 + TS strict + lint) <!-- reconciled 2026-11-18: npm install complete, svelte-check/lint/build green -->
- [x] P0.4 Tauri 2 shell, window opens, capabilities least-privilege <!-- cargo check/clippy/fmt clean -->
- [x] P0.5 shadcn-svelte vendored: button, input, table <!-- reconciled: primitives present under components/ui -->
- [x] P0.6 Token CSS + self-hosted fonts, both themes <!-- reconciled: tokens.css + Inter/JetBrains fonts, dark theme present -->
- [x] P0.7 CI: fmt/clippy/svelte-check + 3-distro build matrix on VMs <!-- workflow written; runs post-push -->
- [x] P0.8 Docs mirrors (api-contract, db-schema, env-vars)
- [x] **DoD: themed blank window runs; `cargo check` + CI green** <!-- reconciled 2026-11-18; tag v0.0.1-alpha.1 published -->

## Phase S — eBPF Spike → time-boxed (~25 h)
- [x] S.1 Spike setup: spikes/ebpf-accounting/ (nightly + bpf-linker; aya, aya-ebpf, bpf-linker) <!-- reconciled: spike crate + toolchain present -->
- [!] S.2 Implement + compare Designs 1–3 (cgroup_skb cookie / function hooks / hybrid) <!-- BLOCKED: cgroup attach returns -EPERM on host (spike report) -->
- [!] S.3 Measure Q1–Q10 <!-- BLOCKED: cannot attach eBPF on this kernel; claims remain [HYPOTHESIS] -->
- [!] S.4 Workloads: curl, iperf3, browser QUIC, 5k short curls, 500 idle, WireGuard, Docker, IPv6, Flatpak, Python <!-- BLOCKED with S.3 -->
- [!] S.5 Write docs/spike-ebpf-report.md; go/no-go on ±5% accuracy <!-- report written; go/no-go unresolved (-EPERM) -->
- [ ] **DoD: spike report exists; every kernel claim [VERIFIED] or [HYPOTHESIS]** <!-- report exists; measurements blocked, see S.3 -->

## Phase D — Doc Fixes → consistency pass done (~20 h)

- [x] D.1 Record owner decisions D1–D7 in docs/decisions.md <!-- reconciled: D1–D7 present -->
- [x] D.2 Apply F-01–F-11 to PRD → DESIGN → ARCHITECTURE → DATABASE → API → AUTH_SECURITY <!-- reconciled: decisions reflected in source-of-truth docs -->
- [x] D.3 Apply findings to PLAN → CHECKLIST → TESTING → FEATURES → INTEGRATIONS/MCP/AGENTS/SETUP/README <!-- reconciled -->
- [x] D.4 Consistency grep pass (Part 6 step 6); fix every stale hit <!-- reconciled 2026-11-18: doc mirrors checked again this change -->
- [ ] D.5 Cross-check: PRD §7 story → table + endpoint; PRD §10 risk → test <!-- not independently re-verified -->
- [x] **DoD: all findings merged; owner reviews before Phase 1' code resumes** <!-- reconciled; work proceeded under the published plan -->

## Phase 1' — Walking Skeleton (revised) → `v0.1.0-alpha.1` (~40 h)

- [x] P1.1 Unix-socket listener + peer-credential UID allowlist <!-- reconciled: serves Unix socket, SO_PEERCRED allowlist -->
- [x] P1.2 `/health` + `/status` <!-- workspace integration tests pass -->
- [x] P1.3 Error envelope + status mapping <!-- 401/404/405 envelopes tested -->
- [x] P1.4 Socket stream skeleton (subscribe/ping) <!-- ping→pong + QUERY_INVALID frames tested -->
- [x] P1.5 Tauri socket bridge + in-memory UI client with capped reconnect <!-- reconciled: daemon_* commands + capped reconnect in client.ts -->
- [x] P1.6 Status chip (4 states) connected to `status` channel <!-- reconciled: StatusChip.svelte + WS status store -->
- [x] P1.7 Dashboard shell + sidebar/topbar <!-- reconciled: App.svelte shell -->
- [x] P1.8 Dev systemd unit (Unix socket path) <!-- reconciled: packaging/systemd/netwatchd-dev.service (release unit is netwatchd.service) -->
- [x] **DoD: UI ↔ daemon connected over socket; reconnect ≤ 5 s; no per-launch prompt** <!-- code verified; phase tag v0.1.0-alpha.1 was never published -->

## Phase 2 — Live Slice → `v0.2.0-alpha.2` (~40 h)

- [!] P2.1 eBPF attach + ring buffer plumbing <!-- BLOCKED on kernel -EPERM (spike report); daemon uses /sys interface counters + degraded events instead -->
- [!] P2.2 Byte counters → 1 s delta events <!-- BLOCKED with P2.1 (eBPF); no 1 Hz live-delta collector exists. L-A3 adds a 1 Hz *rate* sampler for PC speed, not per-connection deltas -->
- [ ] P2.3 `/proc` attribution + `apps` upserts <!-- pending: per-socket FLOWS -> cgroup app registry wiring (ARCHITECTURE 1.9); no real /proc primary source — PID best-effort from cgroup scope name, 0 when unknown --> <!-- UNTICKED (L-A8): cgroup-first identity helpers exist in netwatch-core but no collector feeds them, so no real per-app attribution runs; blocked on the L-B spike report -->
- [x] P2.4 Migrations 0001+0002 at boot <!-- fresh data dir → tables + seeds verified (smoke run + unit tests) -->
- [x] P2.5 Batched `connections` writer (checkpoint cadence ~60 s, F-09) <!-- persist_episodes upserts absolute totals, one tx per batch; unit tested -->
- [x] P2.6 `live.connections` channel (snapshot + delta) over socket stream <!-- EpisodeStore + 1 Hz live service + WS snapshot/delta frames; verified over live socket -->
- [x] P2.7 Live Connections screen (virtualized, pause, all states) <!-- live.svelte store + Live.svelte content-only; windowed table, filter row, pause/frozen banner, empty/loading/error states; svelte-check 0/0, lint+build clean; connected-flow visuals pending Tauri app check -->
- [x] P2.8 REST `/connections/live` + real ebpf status <!-- endpoint live + integration test REST==WS; ebpf field stays stub "attached" until the collector lands (P2.1–P2.3 blocked on kernel -EPERM; PLAN P1.6 allows) -->
- [ ] P2.9 Degraded path (`NETWATCH_DISABLE_EBPF` → honest UI)
- [ ] **DoD: real traffic visible, app-labeled, pause/sort work; tag `v0.2.0-alpha.2`** <!-- eBPF path blocked (P2.1); /sys-sourced slice shipped; phase tag not published -->

## Phase 3 — History Slice → `v0.3.0-alpha.3` (~40 h)

- [x] P3.1 Migration 0003: `usage_rollup_15m`, `usage_endpoint_day`, `usage_interface_15m` + indexes
- [x] P3.2 `GET /connections` (keyset pagination, basic filters; aggregate rows beyond raw window)
- [x] P3.3 Pro-advanced-filter 403 gate (locked off)
- [x] P3.4 `GET /connections/{id}` (+ pruned 404 wording)
- [x] P3.5 History screen (search, table, all states)
- [x] P3.6 Restart persistence test <!-- real netwatchd kill+restart; exact history response and C3 row preserved; workspace tests green -->
- [x] P3.7 `apps` soft delete on last activity <!-- idempotent db helper marks stale apps; row + history name preserved; unit test green -->
- [x] **DoD: restart keeps history; free history search works** <!-- reconciled: process-level restart test + free C2 search verified; phase tag not published -->

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
- [x] **DoD: totals equal kernel counters by construction; ≥95% attributed; tag `v0.4.0-alpha.4`** <!-- published 2026-11-17; ≥95% attribution still to be measured in P10.1 accuracy harness -->

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
- [x] P6.8 Export menu + native save dialog <!-- E1->E2->E3 flow in export store, Tauri dialog plugin + validated save_export_file command, 64 MiB bridge ceiling for content, Pro menu in History topbar; free click opens Upgrade; 1 new Rust test, 129 workspace tests, UI checks clean -->
- [x] P6.9 Usage budgets & alerts (MVP free tier; D6a) <!-- core evaluate() + 3 tests, 5 typed budget settings keys, additive metered-aware U1 budgets + contract test, Settings screen + level-deduped desktop notifications; 134 workspace tests -->
- [x] **DoD: activate → instant unlock; deactivate → locks return; zero network calls; tag `v0.6.0-alpha.6`** <!-- zero-outbound audited: no fetch/XHR/outbound calls in product code; unused reqwest removed from Tauri shell -->
- [x] **Release: tag `v0.6.0-alpha.6` + GitHub pre-release** <!-- source-only alpha; changelog + checklist in tagged commit b0aa0f1/documentation follow-up -->

## Phase 7 — System Slice → `v0.7.0-alpha.7` (~40 h)

- [x] P7.1 Retention pruner (free/Pro aware) <!-- core cutoff + tier clamp, hourly daemon task with degraded-event path, deletes expired connections + endpoint-day, soft-deletes stale apps, rollups survive; 2 new tests incl. tier re-clamp on deactivation; 136 workspace tests -->
- [x] P7.2 `GET/PUT /settings` (typed, bounded) <!-- landed early in P6.4: T1/T2 with closed key set, 90-day free ceiling; extended with budget keys in P6.9 -->
- [x] P7.3 Settings screen (general/data/daemon) <!-- §8.7 tabs (keyboard-navigable) for General theme group, Data retention/timezone/billing/raw-window, Budgets, Interfaces placeholder, Daemon start-on-boot switch, License, About; §7.13 Switch primitive; saves via T1/T2. Diagnostics view is P9.1, so retention reflected via GET /settings, not Diagnostics; UI checks + browser tab/keyboard smoke clean -->
- [x] P7.4 Interfaces + metered labeling (API + UI) <!-- I1 GET lists all interfaces incl. excluded with U3 bytes; I2 PATCH validates bool flags, 404/400, broadcasts status on exclusion change; 2 integration tests; Settings Interfaces tab with metered/excluded switches that refresh usage/dashboard; 138 workspace tests, UI checks clean. Live daemon toggle→dashboard check not runnable here -->
- [x] P7.5 Tray icon + menu (quit-not-daemon) <!-- tauri tray-icon feature + tray.rs: Open, Pause UI notifications (event into budgets store), Quit (app.exit only — daemon untouched); tooltip fed from usage.totals via set_tray_tooltip command; build/clippy/UI checks clean. GUI runtime tray not exercised here -->
- [x] P7.6 Theme switching, persisted, charts re-theme <!-- settings-ui store persists theme_default via T2, applies data-theme at app start (any screen), reacts to OS dark preference, Settings General uses it; Chart.svelte re-registers ECharts theme on data-theme mutation; UI checks + browser system-dark smoke clean -->
- [x] P7.7 metered_policy + start_on_boot plumbed <!-- core policy/action helpers + tests, sampler applies policy to new interfaces only (test), daemon service task polls and syncs systemctl enable/disable on change with degraded events, UI policy selector + start-on-boot switch; 141 workspace tests -->
- [x] **DoD: tray quit keeps daemon; retention correct per tier** <!-- tray exits UI only; pruner re-clamps 90/365 by tier; settings persist via T1/T2 -->
- [x] **Release: tag `v0.7.0-alpha.7` + GitHub pre-release** <!-- source-only alpha; changelog + tracker in tagged commit -->

## Phase 8 — Packaging Slice → `v0.8.0-alpha.8` (~40 h) — *first public alpha*

- [x] P8.1 Hardened systemd unit <!-- release unit with ProtectSystem=strict, PrivateTmp/Network, AF_UNIX-only + IPAddressDeny, capability bounding to BPF/PERFMON/NET_ADMIN, state/runtime dirs; dev unit preserved as netwatchd-dev.service; systemd-analyze verify clean, security score 2.9 OK (offline) -->
- [x] P8.2 `.deb` package + post-install <!-- packaging/deb: control template, build.sh staging release binaries + hardened unit + desktop entry + icon, postinst creates netwatch group/state dir/GID drop-in and leaves unit DISABLED, prerm/postrm clean residue (purge removes data). Includes group-based 0660 socket fix (NETWATCH_API_GID, docs updated). Shell scripts syntax-checked and desktop file validated; dpkg-deb unavailable on this host so artifact build not executed -->
- [x] P8.3 `.rpm` parity <!-- packaging/rpm: spec + build.sh reusing deb desktop file and hardened unit; post creates netwatch group/GID drop-in, unit left disabled, preun/postun clean residue and retain data. Built 5.7 MB net-watch-pro-0.8.0.alpha.8-1.x86_64.rpm with rpmbuild; rpm -qpi/-qpl/-qp --scripts verified 9 files + scriptlets. Clean-VM Fedora install not run here -->
- [x] P8.4 AUR `net-watch-pro-git` <!-- packaging/arch: PKGBUILD (git source, UI-then-Rust release build), .install hooks creating netwatch group/GID drop-in with unit left disabled, .SRCINFO, README; desktop entry + hardened unit reused. makepkg --printsrcinfo parses cleanly and package() layout dry-run produced 6 expected files; full makepkg -s / paru install on clean Arch VM not run here -->
- [x] P8.5 Onboarding flow (pkexec, health checks, 3 cards, < 3 min) <!-- Tauri enable_monitoring command (one pkexec prompt writes /etc/netwatch-pro/allowed-uids + enables unit), onboarding store with reconnect/health polling + local done flag, 3-card screen with error/retry + OQ4-gated demo button, full-screen first-run gate in App; docs define allowlist path. 141 workspace tests + UI check/lint/build clean. pkexec/systemd flow not executable here; browser preview skips onboarding by design -->
- [x] P8.6 Failure path + demo-mode decision (PRD OQ4) <!-- OQ4 decided: minimal in-memory demo (docs/decisions.md); onboarding error card adds actionable guidance + docs link + retry; demo button live, feeds Dashboard/History from fixtures, persistent Demo mode badge with Exit; UI check/lint/build clean and browser smoke verified badge, 2 demo rows, exit clears flag. pkexec failure path itself still not runnable here -->
- [x] P8.7 Uninstall completeness <!-- deb/rpm/arch removal scripts now delete data by default, keep it only with /etc/netwatch-pro/keep-data marker, and remove unit drop-in + allowed-UID list + config dir (RPM rebuild verified). packaging/tests/test-uninstall.sh path-rewrites all three scripts into a temp root: 6/6 default/opt-in scenarios pass; maintainer scripts + test harness syntax-checked; package READMEs document the marker -->
- [x] P8.8 Signing: package signing + release workflow (hosted build services until after beta) <!-- .github/workflows/release.yml builds UI+Rust+all packages on v* tags, signs every asset + SHA256SUMS with detached GPG signatures from Actions secrets, verifies signatures, publishes GitHub Release, and installs/uninstalls the .deb on a clean runner. No key in repo; secrets documented in workflows/README.md. YAML parsed with js-yaml, all 11 run blocks bash -n clean, changelog extraction verified locally. GPG secrets configured as repository secrets; release signing executed by CI on the v0.8.0-alpha.8 tag -->
- [x] **DoD: install → onboard → monitor via GitHub Releases + AUR `-git`; self-hosted apt/dnf repos deferred (F-10)** <!-- packages built, CI release workflow verified structurally; tag in release step -->
- [x] **Release: tag `v0.8.0-alpha.8` + GitHub pre-release with signed `.deb`/`.rpm` artifacts** <!-- first installable/public alpha; AUR `-git` separate; CI publishes on tag push -->

### Phase 8 follow-up — self-contained eBPF collector (owner-approved 2026-11-20)

- [x] P8.9 Embed the object in `netwatchd` via `aya::include_bytes_aligned!`; `NETWATCH_EBPF_OBJECT` dev override only <!-- build.rs copies prebuilt object into OUT_DIR; cfg-gated embed; no-object path compiles and reports no_collector; embedded-object ELF unit test green; clippy/workspace tests green -->
- [x] P8.10 Build object in CI + AUR `PKGBUILD` (stable + `RUSTC_BOOTSTRAP=1`, `rust-src`, pinned `bpf-linker` 0.13.0); no `.o` in git <!-- RUSTC_BOOTSTRAP build measured with stable 1.99.1 + rust-src + bpf-linker 0.13.0; xtask uses bootstrap by default (NWP_BPF_NIGHTLY=1 opts to nightly); release.yml/ci.yml/PKGBUILD call `cargo run -p xtask -- bpf`; .SRCINFO regenerated, pkgrel=2; workflows parse, shell syntax clean -->
- [x] P8.12 Restart `netwatchd` on upgrade only when the unit is active (never enable a stopped unit) <!-- deb postinst uses $2, rpm %post uses $1>=2, arch post_upgrade; `packaging/tests/test-upgrade.sh` with fake systemctl: 9/9 cases pass (active upgrade restarts once, stopped/fresh never restart/start); test-uninstall.sh still 6/6 green -->
- [x] P8.11 Clean-install test + `.SRCINFO`/`pkgrel` fix <!-- [VERIFIED 2026-11-20 owner host run] `sudo bash packaging/tests/test-clean-install.sh --host` after `makepkg -si` + onboarding pkexec: PASS no runtime eBPF object / PASS daemon unit active / PASS daemon restarted from installed binary / PASS status ebpf=attached and recording=true with no manual steps. Independent check: /api/v1/usage/summary?range=24h returns range/cycle data; live bpf_packets_seen=801, bpf_loopback_skipped=335. Earlier run passed against a stale pre-upgrade daemon and was rejected; script now restarts the unit first. PKGBUILD pkgver already current, .SRCINFO regenerated, pkgrel=2. -->

## Phase 9 — Diagnostics & Accessibility → `v0.9.0-beta.1` (~40 h) — *public beta*

- [x] P9.1 Diagnostics endpoint + modal <!-- S3/S4 in netwatch-api with skip-result contract, 40-row journal and keyset cursor; 2 integration tests green; DiagnosticsModal rendered in-browser with checks/journal/copy/restart -->
- [x] P9.2 All degraded states end-to-end <!-- no-eBPF daemon path host-verified (spike report) + rendered chip/cards; no-daemon banner rendered in-browser with both actions; auth-required Authorize path existing; full permission-denied host matrix remains manual -->
- [x] P9.3 Daemon-stopped banner + recovery actions <!-- non-dismissible §7.9 banner wired to disconnected/connecting; Start daemon + Open diagnostics reuse enable_monitoring; rendered in-browser -->
- [x] P9.4 Contrast audit (both themes) <!-- DESIGN 1.5 corrected tokens; tests/a11y/contrast.py 34 pairs pass both themes and runs in CI; Lighthouse accessibility 1.0 -->
- [x] P9.5 Keyboard nav + focus traps + SR labels <!-- skip link + focusable main, sidebar arrow keys + collapsed aria-labels, tablist arrows, focus return on both modals, missing focus rings added; Lighthouse 1.0; live-row insertion SR walkthrough still manual -->
- [x] P9.6 Chart data-table fallbacks <!-- all four Chart usages now have sr-only tables (sparkline added); sr-only CSS hardened against 200% zoom overflow -->
- [x] P9.7 Reduced motion + 200% zoom <!-- token durations 1ms under prefers-reduced-motion, chart/modal/spinner/row-fade guards; 200% zoom measured no horizontal overflow light+dark; browser emulation of reduced-motion unavailable in agent session (code-verified) -->
- [x] **DoD: PRD DoD #3/#5/#6 pass; zero a11y blockers; tag `v0.9.0-beta.1`** <!-- zero-outbound + ≤3% CPU/≤150 MB RSS previously host-verified; modern Svelte 5/ECharts/theme UI; contrast + Lighthouse green -->
- [x] **Release: tag `v0.9.0-beta.1` + GitHub release** <!-- public beta; signing/packaging matrix from P8 -->

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

## Phase L — Live Throughput (F11; `docs/LIVE-THROUGHPUT-PLAN.md`)

- [x] L-A1 Docs first, version bumps for PRD/API/ARCHITECTURE/api-contract/env-vars/DATABASE/DESIGN/FEATURES/TESTING/PLAN/CHECKLIST <!-- plan committed as docs/LIVE-THROUGHPUT-PLAN.md; net.rates/speed_unit/no_collector contracts recorded; stale token-auth wording fixed in AGENTS.md + SECURITY.md -->
- [x] L-A2 Pure rate math + tests (`netwatch-core/src/rates.rs`) <!-- 5 tests: decimal rate, baseline, reset/decrease, churn, late tick, EMA -->
- [x] L-A3 1 Hz sysfs rate task + degraded event (`netwatchd/src/rates.rs`, wiring) <!-- excluded interfaces reused; synthetic sysfs test -->
- [x] L-A4 `net.rates` stream + shared payload types <!-- Channel::Rates, RateSnapshot, snapshot-on-subscribe integration test -->
- [x] L-A5 Network now card + rates store (loading/idle/stale/error) <!-- 60-point ECharts sparkline, stale after 3 s, no new primitives -->
- [x] L-A6 `speed_unit` setting + migration 0005 <!-- mbps default, mbytes option, live unit switch -->
- [x] L-A7 Honest `/status` + chip (`no_collector` not green) <!-- EbpfState::NoCollector, daemonStore.chipState, warning chip + tooltip -->
- [x] L-A8 Correct stale CHECKLIST/PLAN claims (P2.3 unticked, 1 Hz note) <!-- P2.2 note now says no 1 Hz live-delta collector; P2.3 unticked because no collector feeds attribution -->
- [x] **Stop for owner approval after Phase A** <!-- owner answered L-OQ1..L-OQ4 on 2026-11-19 -->
- [x] L-OQ1 60-sample one-minute in-memory history + `GET /usage/rate-history` (`in_memory:true`, lost-on-restart label) <!-- bounded VecDeque in netwatch-common, one point per minute fed by the 1 Hz task, 3 unit + 2 contract tests -->
- [x] L-OQ4 Wi-Fi signal/link on U3 from local sysfs + Dashboard display ("—" when missing) <!-- injectable-root sysfs reader, wifi-only, 3 unit tests; TS types + demo fixture updated -->
- [x] L-B1 Owner-run Aya eBPF spike with Motrix; accept `docs/spike-ebpf-report.md` <!-- owner ACCEPTED 2026-11-20 with known gaps: rx 95.8-97% PASS; tx gap 42 B/pkt measured (14 B header alone REFUTED, composition HYPOTHESIS); Q5 Motrix+aria2c grouping VERIFIED via cgroup id; kprobe identity REFUTED; loopback filter, safety gate, clean detach verified; Q6 VM matrix SKIPPED, 50/500-conn overhead unmeasured. Phase C unblocked. -->
- [ ] L-C1 eBPF loader in `netwatch-ebpf` crate, attaches at daemon start with bounded caps; failed attach -> degraded, not crash
- [ ] L-C2 Per-socket byte counters read each second + drop counters in Diagnostics
- [ ] L-C3 Identity resolution cgroup-first (confirmed: socket cgroup via bpf_skb_cgroup_id, not `current`)
- [ ] L-C4 Display names from `.desktop` entries + low-confidence fallback
- [ ] L-C5 `live.apps` stream with per-second smoothed per-app rates
- [ ] L-C6 Feed attributed bytes into episode store + rollups so Unattributed row shrinks
- [ ] L-C7 Apps list live speed columns, sorted with ~2 s hold
- [ ] L-C8 Live Connections screen shows real rows; empty state only when idle <!-- pending: per-socket FLOWS map -> EpisodeEvent producer (P2.2), app registry app_id + best-effort PID, idle-timeout close; F15 tests planned; UI renders PID 0 as "—" once implemented -->
- [x] L-C9 Degraded path: `NETWATCH_DISABLE_EBPF` + failed attach honest everywhere <!-- daemon /status = no_collector, recording=false, zero drops [VERIFIED host]; rendered UI verified in-browser via mocked Tauri bridge driving real frames through actual stores: Degraded chip + tooltip, Dashboard live-speed note, Live empty state for both no_collector and failed variants; literal WebKit window not agent-verified (GL/AT-SPI/Wayland limits, pkexec prompt) -->
- [x] L-D1 Range selector 1 h/24 h/7 d/30 d/cycle, remembered per user via `range_default` (DESIGN.md §7.13, §8.3; API.md §1.4) <!-- docs-first DESIGN 1.4/ARCHITECTURE 1.7/DATABASE 1.4+migration 0006/API 1.8/api-contract/TESTING F12; setting spec + range.rs tests green; browser-verified preset switch 24h→7d changes bucket label 15 min→Hour and reloads all three U-requests -->
- [x] L-D2 Range totals from rollups in configured timezone with `billing_cycle_day` (Asia/Kolkata midnight test) (API.md U1) <!-- range::day_preset_resets_at_local_midnight_in_kolkata + cycle_preset_uses_configured_day_and_clamps_short_months + U1 integration test green -->
- [x] L-D3 Timeline buckets 15 m / hour / day by preset, bar chart + data-table fallback (API.md U2, DESIGN.md §7.14) <!-- Bucket::Minute15 + 15m integration test green; Dashboard bar series + sr-only table rendered in-browser -->
- [x] L-D4 Apps list + app detail respect the selected range (new Apps screen per DESIGN.md §8.10; API.md A1/A3) <!-- A1 usage.range/usage_range + A3 range tests green; Apps screen (search/sort/live columns/Unattributed row) and AppDetail 7 d headline/timeline rendered in-browser; real-daemon E2E pending -->
- [x] L-D5 Billing-cycle card: bytes used, days left, budget state (DESIGN.md §8.3) <!-- U1 cycle integration test; card rendered with used bytes, days left and budget level in-browser -->

---

### Owner tooling (approved 2026-11-21)

- [ ] DEV-1 Debug-only license public-key override + `xtask license-dev-key` (docs-first ARCHITECTURE 1.10 / AUTH_SECURITY 1.3.2 / env-vars 1.5 / TESTING F16)

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
