# TESTING.md — Test Plan

**Version:** 1.1 · **Date:** 2026-11-15  
**Sources:** `PRD.md` §9/§11 (metrics, DoD) · `FEATURES.md` (per-feature behavior + acceptance criteria) · `API.md` §1–2 (contracts, error codes) · `ARCHITECTURE.md` §4/§9 (tests/ layout, budgets) · `docs/decisions.md` (owner decisions D1–D7)

**Version history:** 1.1 (2026-11-15): apply plan-fix findings — accuracy harness redefined (F-03/D3a), auth matrix → UID allowlist (F-06/D1a), timezone/billing-cycle cases (F-04/D4a), cgroup identity cases (F-05/D5a), kernel VM matrix not containers (F-08), write-rate test (F-09).

> **Working rule (binding):** tests are generated **for one feature at a time**, together with the feature code, and **must pass before the next feature starts** — same gate as FEATURES.md §11 (build → test green → update PLAN/CHECKLIST → user confirmation).

---

## 1. Tools per Layer

| Layer | Tool | One-line reason |
|---|---|---|
| Rust unit tests | `cargo test` (built-in) + `tokio` test runtime + `tempfile` | Zero extra frameworks; async tests run on Tokio exactly like production. |
| Rust property tests | `proptest` (dev-dep, core formatters/parsers only) | Randomized input for byte-formatting, keyset cursors, filter parsing. |
| Rust coverage | `cargo-llvm-cov` | Standard, fast, CI-friendly Rust coverage. |
| Mocks (Rust) | Hand-rolled fakes + `mockall` (dev-dep) for traits | Traits are few (future `PaymentsProvider`, clock, collector); mockall is the popular default. |
| API/integration tests | Rust integration tests in `tests/integration/` driving the real daemon binary + temp SQLite | Tests the真实 process boundary — real migrations, real token file, real WS. |
| DB tests | sqlx test macros against temp DBs + `VACUUM INTO` snapshots | Forward-only migrations exercised exactly as in production (DATABASE.md §8). |
| UI unit tests | Vitest + `@testing-library/svelte` | Standard Svelte 5 stack; stores/components tested with runes. |
| UI e2e | Playwright (already in MCP.md §3.4) driving the `tauri dev` webview | Real app, real WS, real states — screenshots double as DESIGN.md review artifacts. |
| Accuracy harness | Custom binary in `tests/accuracy/` + synthetic traffic generator | Proves PRD accuracy: totals equal kernel interface counters by construction; ≥95% of bytes attributed (DoD #2) repeatably in CI. |
| Performance | Perf assertions inside integration tests (CPU/RSS sampled) | Budgets are gates, not vibes (PRD ≤3% CPU / ≤150 MB RSS). |
| Static gates | `cargo fmt --check`, `cargo clippy -D warnings`, `svelte-check`, eslint, `cargo deny`, `npm audit` | Already mandated by ARCHITECTURE.md §8.2 / P0.7. |

**Dev-dependency note:** `proptest`, `mockall`, `cargo-llvm-cov`, `reqwest` (test client), `@testing-library/svelte`, `vitest` are new dependencies → added per feature with user approval (AGENTS.md §5), tracked in this file.

---

## 2. Priorities (critical paths first)

| P | What | Why |
|---|---|---|
| **P0** | Accounting accuracy (F2/F3 harness; totals equal kernel counters, ≥95% attributed); socket peer-credential auth matrix (UID in/out of allowlist); license gating server-side (F9); socket reconnect ≤ 5 s (F1); persistence across restart (F5); migration forward-only integrity | These are the product's promises; a failure here is a launch blocker (PRD §11). |
| **P1** | Feature acceptance criteria F1–F10 (§4 tables); error/loading/empty states; pagination/sort/filter contracts; rate limits | Core UX correctness per API.md contracts. |
| **P2** | Edge cases (§5); i18n-ish formatting; theme switching; virtualization at scale; reduced-motion | Polish-level; still tested, later in each feature. |

---

## 3. Test Data & Mocking Strategy

| Need | Strategy |
|---|---|
| Synthetic traffic | **Test collector** (`--feature-flag test-collector` build of the daemon, never shipped): replays recorded eBPF delta sequences from `tests/fixtures/traffic/*.jsonl` — deterministic, no kernel needed in CI. |
| Process attribution | Fixture `/proc` trees (fake pids/exe paths) mounted via test config; confidence levels exercised exactly (exact/heuristic/unknown). |
| History volumes | `tests/fixtures/db/seed_*.sql`: 10 k rows (UI virtualization), 90-day spread (pagination), 1-year soak profile (P10.3), empty db, single-row db. |
| Clock | Injectable `Clock` trait in `netwatch-core`; tests pin `now` — rollup buckets, retention pruning, month boundaries all deterministic (F2 timezone cases). |
| Daemon down / errors | Kill/deferred-start in integration tests; UI e2e uses a **mock socket server** serving canned snapshots/deltas/errors from fixtures (F1 frozen/disconnect states). Auth is peer-credential — tests run as allowed UID vs denied UID. |
| Kernel matrix | **VMs** (QEMU/virtme-ng/Vagrant) for Arch/CachyOS, Ubuntu LTS, Fedora — containers share the host kernel and cannot catch kernel-specific eBPF failures (F-08). Containers only for userspace tests. |
| Rate limits | Integration tests fire 6th export / 121st read and assert 429 + `Retry-After` shape. |
| Theme/screenshots | Playwright runs in light+dark; baseline screenshots stored per breakpoint (960/1280/1600 widths) for DESIGN.md §5.4 reflow review. |

---

## 4. Test Case Tables per Feature

### F1 — Live Connections view

| Scenario | Input | Expected |
|---|---|---|
| New connection appears | fixture delta: open tcp conn | Row ≤ 2 s, correct app label + icon, mono rates |
| Same-tick open+close | delta with add then remove, same id | Row removed; no ghost, no crash; add-before-remove order held |
| Unknown attribution | connection with no app match | Fallback icon + "Unknown process", confidence `unknown` |
| IPv6 + ICMP rows | `::1` conn, icmp conn | Compressed IPv6 renders; port `—` for icmp; no column jitter |
| Sort/filter | `?sort=rate`, `?protocol=udp` | Ordered per contract; unknown filter → 400 `QUERY_INVALID` shown |
| Pause | click freeze | Banner "Frozen at HH:MM"; WS deltas ignored until resume |
| Daemon killed mid-stream | stop daemon | Chip "Connecting…", data marked stale (caption), recovers ≤ 5 s over socket reconnect |
| Degraded mode | `NETWATCH_DISABLE_EBPF=1` | Warning chip + banner; table opens empty with hint |
| Auth: UID in allowlist | run UI as allowed UID | Socket connect succeeds; endpoints return 200 |
| Auth: UID not in allowlist | run UI as non-allowed UID | Socket connection refused; chip "Stopped" |

### F2 — Per-app usage accounting

| Scenario | Input | Expected |
|---|---|---|
| Daily totals | accuracy harness, 50-conn synthetic load | Headline total equals kernel interface counters by construction; ≥95% of bytes attributed; unattributed row shows the gap |
| Compaction preserves totals | force raw window expiry | raw+rollup sum constant across the seam |
| Soft-deleted app | expire last activity, query history | Name/icon still resolve; `deleted_at` set |
| Renamed binary | same path, new hash | New `apps` row; old history untouched |
| Month boundary | clock pinned at UTC month edge | "This month" excludes last month's tail; tz caption correct |
| Clock skew/DST | pinned offsets ±1 h | UTC buckets stable; local labels shift visibly |
| Half-hour/45-min TZ | `Asia/Kolkata` (+5:30), `Asia/Kathmandu` (+5:45) | "today" resets at local midnight, not 05:30/05:45 |
| Billing-cycle day | set `billing_cycle_day` ≠ 1 | "this month" = billing cycle, not calendar month |
| Top ranking vs summary | query `/apps` + `/usage/summary` | Order and totals agree |
| top_limit bounds | `?top_limit=99` | 400 `QUERY_INVALID` (max 10) |

### F3 — Dashboard

| Scenario | Input | Expected |
|---|---|---|
| Timeline continuity | data spanning 7-day seam | Single series, no double-count (totals constant) |
| Day/Hour toggle | click tabs | Re-query per granularity; no fake interpolation |
| Excluded interface | `lo` traffic only | Never appears in totals or `/usage/interfaces` |
| Metered badge | `is_metered=1` interface | Badge shows; informational only |
| Chart decimation | > 2 k points fixture | Rendered points ≤ cap; shape preserved |
| Theme switch mid-view | toggle dark | ECharts re-registers; series colors match §3.3 |
| Empty vs zero | fresh db / zero-traffic day | Empty state first, zero-cards only after first sample |
| Data-table fallback | open a11y table | Values match chart series exactly |

### F4 — App detail

| Scenario | Input | Expected |
|---|---|---|
| Tabs populated | app with full history | Overview/Connections/Endpoints all real data |
| Soft-deleted app | open deleted app | Renders with "No longer active — last seen …" caption |
| Pruned window | old rows expired | Connections tab shows retained only + retention caption |
| Cgroup identity | Flatpak, Snap, script interpreter, systemd service | `identity_kind` set; `app_key` unique; no exe_path collapse |
| Deleted binary | `/proc/<pid>/exe` shows `(deleted)` | `exe_path` stored as provenance; `app_key` remains stable |
| No icon | app without icon path | Fallback container, never broken image |
| Endpoints cap | app with 300 remote IPs | Top 100 + "and N more" |
| 404 | unknown id | 404 with honest message |
| Click-through | from Dashboard bar / Live row | Correct id lands |

### F5 — Local history

| Scenario | Input | Expected |
|---|---|---|
| Keyset pagination walk | 10 k rows, pages of 50 | No dup/skip within a stable snapshot |
| Basic filter (free) | `app_name=fire` | Matches substring, escaped `%`/`_` |
| Advanced filter (free) | `since=…` | 403 `EXPORT_PRO_REQUIRED` + lock UI |
| Advanced filter (Pro) | same with Pro token | 200; combined filters AND-ed |
| Pruned row fetch | expired id | 404 "outside retention window" |
| Sort whitelist | `?sort=remote_port` | 400 (not in whitelist) |
| Cursor invalidation | prune under held cursor | Next page skips gracefully; `has_more=false` clean |
| Retention prune | 91-day-old rows, free tier | Rows gone; rollups survive; `daemon_events` note |
| Restart persistence | kill + restart daemon | History identical (checksum compare) |
### F6 — Daemon + install experience

| Scenario | Input | Expected |
|---|---|---|
| Fresh onboarding | clean VM + package | Enable → live data < 3 min (all 3 package paths) |
| pkexec cancelled | user denies prompt | Failure card with reason + retry; no silent loop |
| No BTF kernel | simulated kernel check fail | `EBPF_UNSUPPORTED_KERNEL` → guidance + degraded path |
| Token file unreadable | chmod 000 token file | `AUTH_INVALID` → banner, not crash |
| Daemon down banner | stop unit | Non-dismissible banner + Start/Open-diagnostics actions |
| Recovery | start via banner action | Chip returns to Monitoring ≤ 5 s |
| Diagnostics payload | trigger eBPF fail | Checks + journal excerpt + copy-report correct |
| Uninstall | remove package | Zero service residue; data opt-in honored |

### F7 — Tray integration

| Scenario | Input | Expected |
|---|---|---|
| Tray quit | menu → Quit, confirm | UI exits; `/health` still 200 (daemon alive) |
| Also stop monitoring | checkbox + pkexec | Daemon stops; cancel leaves it running |
| Tooltip | wait 5 s with traffic | Today-total updates (mono); degraded → state text |
| No-tray DE | GNOME default env | App functional; docs hint shown in Settings |

### F8 — Settings

| Scenario | Input | Expected |
|---|---|---|
| Retention change | set 120 (Pro) | 200; pruner honors; persists across restart |
| Retention > 90 (free) | set 120 free tier | 403 → upgrade modal; input reverts |
| Wrong type | `{"value": "abc"}` on int key | 400 with field name; nothing half-applied |
| Unknown key | `PUT /settings/bogus` | 400 |
| Metered toggle | switch interface | Dashboard card reflects immediately |
| Theme change | light→dark | Live apply; charts re-theme; survives restart |
| Clear data | destructive confirm | Confirm names consequence; executes; no undo offered |

### F9 — Freemium mechanics

| Scenario | Input | Expected |
|---|---|---|
| Valid key (offline) | signed test key | `tier=pro`; entitlements flip; no network packet seen |
| Tampered key | bit-flipped signature | 403 `LICENSE_INVALID` (distinct from expired) |
| Expired key | expired test key | 403 with expiry message |
| Malformed key | `NWP2-…` wrong prefix | 400 (format) before signature work |
| Double activate | activate twice | 409 `CONFLICT`, no state change |
| Deactivate mid-export | start export, then DELETE license | Job completes; new ones 403 |
| Export CSV/JSON | filtered query | Content matches rows exactly; headers per scope |
| Rate limit | 6th export/min | 429 + `Retry-After`, §1.3 body |
| Duplicate webhook (future) | same event id twice | Second is a no-op (§4.2 contract) |

### F10 — Updates & packaging

| Scenario | Input | Expected |
|---|---|---|
| Install matrix | deb/rpm/AUR on 3 distro families | Working app each path |
| Upgrade path | v0.8 → v0.9 package update | Migrations forward-only; history intact |
| Version skew | UI newer than daemon | Honest "updating" state; destructive ops refused |
| Signature check | tampered artifact | Package manager rejects (CI verifies) |
| Migration failure | corrupted db at boot | Distinct exit code → degraded UX, no partial schema |
| Uninstall opt-in | remove with "keep data" | Service gone; data dir intact; without → gone |

---

## 5. Edge & Negative Test Matrix (cross-feature)

| Class | Cases | Where |
|---|---|---|
| Invalid input | unknown params → 400 naming field; bad types; out-of-range limits; `since>until`; oversized body (>16 KB) | API contract tests, all write/query endpoints |
| Unauthorized | UID not in allowlist → socket refused; allowed UID → 200 everywhere but `/health`; WS without valid UID → refused | Socket auth matrix test (P0) |
| Forbidden | free tier: advanced filters/export/retention>90 → 403 with correct code distinction | Gating matrix (P0) |
| Network failure | daemon killed; WS drop mid-snapshot; token file gone; API addr refused | Integration + e2e (F1/F6 states) |
| Storage failure | read-only data dir; disk-full write; corrupted db page → fail-fast exit codes | Integration fault-injection |
| Kernel failure | attach failure; ring-buffer overflow → drop counter + honest Diagnostics | F1 degraded tests |
| Rate limiting | 429 on read/export/write scopes; `Retry-After` honored by UI | API.md §1.5 tests |
| Concurrency | two UI windows writing settings; WS delta during pagination walk; pruner during export read | Integration race tests |
| Unicode/locale | app names with unicode/emoji; RTL labels in UI; `strftime` locale independence | F2/F4 unit tests |
| Time | DST edges, leap-day, month-lengths (proptest on bucket math) | F2 property tests |
---

## 6. Coverage Targets & Commands

### Targets

| Scope | Target | Enforced |
|---|---|---|
| `netwatch-core` (accounting, attribution, retention, license) | **≥ 90% line** | CI gate — this is the correctness core |
| `netwatchd` + `netwatch-api` | **≥ 80%** | CI gate |
| `netwatch-common` | ≥ 90% (small, pure) | CI gate |
| TS stores/formatters (`lib/stores`, `lib/utils`) | ≥ 85% | CI gate |
| Svelte components | best-effort (state coverage via e2e) | tracked, not gated |
| Overall (Rust) | ≥ 80% | reported per PR |

### Commands (single source — also mirrored into `docs/env-vars.md` sibling `docs/testing.md` when scaffolded)

```bash
# Rust: everything
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                      # unit + integration
cargo llvm-cov --workspace --html           # coverage report

# UI: everything (from app/)
npm run lint && npm run svelte-check
npm run test                                # vitest unit + component
npm run test:e2e                            # playwright (light+dark, 3 widths)

# Accuracy & budgets (gates for DoD)
cargo run -p accuracy-harness -- --profile 50conn   # totals equal kernel counters, ≥95% attributed (P10.1)
cargo run -p accuracy-harness -- --budget           # CPU/RSS p95 (P10.2)

# Full pre-release gate (CI = same order)
just gate   # fmt+clippy+test+cov+lint+svelte-check+vitest+playwright+harness
```

### CI wiring (extends P0.7)

- **On every PR:** fmt, clippy, unit tests, svelte/vitest, contract tests — minutes, cheap.
- **On merge to main:** + integration suite, e2e (light+dark), coverage gates.
- **Nightly:** 3-distro matrix on VMs, accuracy + budget harnesses, soak profile, `cargo deny`/`npm audit`.

---

## 7. Manual QA Checklist (pre-release; per PRD §11 + FEATURES.md)

Run on clean VMs (Arch, Ubuntu LTS, Fedora) with the packaged build. Tick per distro.

**Install & onboarding**
- [ ] Install via each channel works; no dependency surprises
- [ ] Onboarding completes < 3 min; pkexec prompt once (UID allowlist registration); failure paths show guidance
- [ ] Uninstall leaves no residue; data opt-in works both ways

**Core flows**
- [ ] Dashboard numbers plausible vs `vnstat`/interface counters (totals equal kernel counters; unattributed row shows gap)
- [ ] Live view labels apps correctly during: browsing, downloads, streaming, VPN on/off
- [ ] History answers "what used data yesterday/last week"; pagination feels instant
- [ ] App detail tabs all populated for top 5 real apps + 1 unknown process
- [ ] Pro: activate test key → instant unlock → export → deactivate → locks return
- [ ] Socket auth: allowed UID connects; non-allowed UID refused; no per-launch pkexec prompt

**States & honesty**
- [ ] Kill daemon → banner + chip honest; recovery ≤ 5 s, no stale-data lies
- [ ] Empty db first-run shows empty states, never fake zeros
- [ ] Diagnostics shows real checks + journal; copy-report works

**UI quality (DESIGN.md)**
- [ ] Light+dark: tokens consistent, no hardcoded colors spotted (devtools audit)
- [ ] Keyboard-only walkthrough of every screen; focus rings visible everywhere
- [ ] Screen-reader pass: chip/status/toasts/tables announce correctly
- [ ] 200% zoom + reduced-motion honored; 960 px window usable, no clipping
- [ ] Mono numerals right-aligned everywhere; no column jitter in Live

**Trust**
- [ ] Live view shows zero non-loopback sockets **from the app itself**
- [ ] No secrets in logs; token/license never echoed; logs clean on failures
- [ ] `systemd-analyze security` score unchanged from P10.5 review

**Sign-off:** all boxes ticked on all 3 distros → PRD §11 DoD met → release tag may cut.

---

## 8. Per-Feature Test Generation Gate

For each feature F1→F10 (FEATURES.md order):

1. Write the feature code and its tests in the **same change** (unit + integration + e2e rows from §4 table).
2. Run the relevant subset: `cargo test` / `npm run test` / targeted Playwright spec — **all green**.
3. New dev-deps used by the tests get user approval at that moment (AGENTS.md §5).
4. Update PLAN.md/CHECKLIST.md (tick task + add "tests green" note), FEATURES.md §0 status.
5. **Stop. Wait for user confirmation.** A failing test blocks moving on — no "will fix later" parking.

---

*End of TESTING.md. Tests are part of the feature's Definition of Done, not a follow-up.*
