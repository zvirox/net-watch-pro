# FEATURES.md — MVP Feature Build Guides

**Version:** 1.1 · **Date:** 2026-11-15  
**Sources:** `PRD.md` §6.1 (the 10 MVP features) · `PLAN.md`/`CHECKLIST.md` (task mapping) · `DESIGN.md` (UI) · `API.md`/`DATABASE.md` (contracts) · `docs/decisions.md` (owner decisions D1–D7)

**Version history:** 1.1 (2026-11-15): apply plan-fix findings — Unix-socket + peer-credential (F-06/D1a), endpoint-day aggregates (F-02/D2a), interface-counter totals + Unattributed row (F-03/D3a), 15m rollup grain + timezone/cycle (F-04/D4a), cgroup-first identity (F-05/D5a), budgets/alerts in MVP free tier (D6a).

> **Working agreement (binding):**
> 1. Build **one feature at a time**, top to bottom in the order below.
> 2. After finishing a feature: update `PLAN.md` + `CHECKLIST.md` (tick task boxes, tag if the phase completes), then **stop and wait for the user's confirmation** before starting the next.
> 3. Each feature's build order maps to PLAN.md task IDs — do not invent tasks; if a gap appears, add it to PLAN.md first.
> 4. Prerequisite: Phases 0–1 of PLAN.md (foundation + walking skeleton) are complete before Feature 1 starts.

---

## 0. Feature ↔ Plan ↔ Contract Cross-Reference

| # | Feature | PRD | PLAN phase | Key API | Key DB tables | Screens (DESIGN.md §8) |
|---|---|---|---|---|---|---|
| F1 | Live Connections view | §6.1 #1 | Phase 2' | `WS live.connections`, `GET /connections/live`, `/status` | `connections`, `apps` | Live (§8.4) |
| F2 | Per-app usage accounting | §6.1 #2 | Phase 2'–4' | `GET /usage/summary`, `/apps`, `/apps/{id}/usage` | `usage_rollup_15m`, `usage_interface_15m`, `connections`, `apps` | Dashboard, App detail |
| F3 | Dashboard | §6.1 #3 | Phase 4' | `GET /usage/summary`, `/usage/timeline`, `/usage/interfaces`, `WS usage.totals` | `usage_rollup_15m`, `usage_interface_15m`, `interfaces` | Dashboard (§8.3) |
| F4 | App detail page | §6.1 #4 | Phase 5 | `GET /apps/{id}`, `/apps/{id}/usage`, `/apps/{id}/endpoints` | `apps`, `connections`, `usage_endpoint_day` | App detail (§8.5) |
| F5 | Local history | §6.1 #5 | Phase 3' (+P7.1 prune) | `GET /connections`, `/connections/{id}` | `connections`, `usage_endpoint_day` | History (§8.6) |
| F6 | Daemon + install experience | §6.1 #6 | Phases 1', 8', 9 | `GET /health`, `/status`, `/diagnostics`, `/events` | `daemon_events`, `settings` | Onboarding (§8.2), Diagnostics (§8.9) |
| F7 | Tray integration | §6.1 #7 | Phase 7 | `WS usage.totals` (cached) | — (reads cache) | Tray (no screen) |
| F8 | Settings (lean) | §6.1 #8 | Phase 7 | `GET/PUT /settings`, `PATCH /interfaces/{id}` | `settings`, `interfaces` | Settings (§8.7) |
| F9 | Freemium mechanics | §6.1 #9 | Phase 6 | `/license*`, `POST /exports`, gating on C2/T2 | `license` | Upgrade modal (§8.8), History locks |
| F10 | Updates & packaging | §6.1 #10 | Phase 8', 11 | `GET /health` (version) | `schema_migrations` | Onboarding, About |

Phase 1 status note: Phase S (eBPF spike) and Phase D (doc fixes) are in progress before Phase 1' code resumes. F6 remains not started pending socket + peer-credential implementation (D1a).

Status tracker (update after each confirmed feature):

| # | Feature | Status |
|---|---|---|
| F1 | Live Connections view | ☐ not started |
| F2 | Per-app usage accounting | ☐ not started |
| F3 | Dashboard | ☐ not started |
| F4 | App detail page | ☐ not started |
| F5 | Local history | ☐ not started |
| F6 | Daemon + install experience | ☐ not started (skeleton exists from Phase 1) |
| F7 | Tray integration | ☐ not started |
| F8 | Settings (lean) | ☐ not started |
| F9 | Freemium mechanics | ☐ not started |
| F10 | Updates & packaging | ☐ not started |

---

## 1. F1 — Live Connections View

### What & story
Real-time table of every active connection: app name/icon, PID, protocol, remote endpoint, current rate, cumulative bytes; freeze/pause, sort, filter (PRD §6.1 #1).
Covers: *"As a privacy-conscious user, I want to see every app's live network connections in one view"* + the attribution story (PRD §7, stories 1–2).

### Screens & components (DESIGN.md)
- **Live screen** (§8.4): topbar with search + pause, filter row, virtualized table.
- Components: `StatusChip` (§7.5), tables (§7.6 — mono right-aligned numerals), search input (§7.2 pill), filter selects, freeze banner (§7.9), empty state (§7.11), skeleton rows (§7.10).

### Data contracts
- **DB:** `connections` (`is_open=1` rows mirror in-memory live set), `apps` (labels/icons).
- **API:** `WS live.connections` (snapshot + id-keyed deltas ≤1 msg/s), `GET /connections/live`, `GET /status` (ebpf state).

### Build order
1. eBPF attach + ring buffer → byte deltas 1 s (PLAN P2.1–P2.2)
2. `/proc` attribution + `apps` upserts (P2.3)
3. Migrations 0001/0002 + batch writer (P2.4–P2.5)
4. WS snapshot/delta channel (P2.6) + REST snapshot parity (P2.8)
5. Live screen UI: virtualized table, in-place rate updates (P2.7)
6. Pause/freeze + degraded path (P2.9)

### Edge cases & validation
- **Row churn:** connection opens+closes within one flush → row may appear with both bytes and `removed` in the same delta batch; apply adds before removes, keyed by `id` (idempotent).
- **Unknown attribution:** `attribution_confidence = heuristic|unknown` → show fallback icon container + name "Unknown process"; never blank rows.
- **IPv6:** render compressed form; column width reserved (mono), no jitter.
- **ICMP/port 0:** port renders as `—` (still right-aligned mono).
- **Disconnect mid-stream:** chip → "Connecting…", table keeps last frozen data with a stale-timestamp caption — never silently looks live (DESIGN.md Principle 3).
- **Validation:** filter params whitelist-only (API.md §2.2 C1); unknown filter → 400 `QUERY_INVALID`.

### States
- **Loading:** skeleton rows matching final layout (no spinners for lists).
- **Empty:** "Waiting for first data" empty state with live hint (never zeros-as-truth).
- **Error:** error-soft banner row with retry (API error codes → ARCHITECTURE.md §7.3 mapping).

### Acceptance criteria
- [ ] Rows appear ≤ 2 s after opening a connection, with correct app label
- [ ] Rates update in place; no full-table re-render jitter; >500 rows virtualized
- [ ] Pause freezes with "Frozen at HH:MM" banner; resume continues
- [ ] Sort/filter within view work; unknown filter → visible 400 handling
- [ ] Daemon killed → chip Connecting, banner appears, recovers ≤ 5 s
- [ ] `NETWATCH_DISABLE_EBPF=1` → Degraded chip + banner, table still opens (empty)

---

## 2. F2 — Per-App Usage Accounting

### What & story
Data used per application for today / this week / this month, up/down split, ranked "Top apps" list, total system usage (PRD §6.1 #2).
Covers: *"As a metered-connection user, I want to see today's and this month's data usage total"* + per-app ranking story (PRD §7).

### Screens & components
- **Dashboard** (§8.3): headline totals (mono `data-lg`), Top apps ranked list.
- **App detail** (§8.5): per-app usage stat cards.
- Components: usage bars (§7.14, mono % text), sparkline, cards (§7.3).

### Data contracts
- **DB:** `usage_rollup_15m` (compacted, 15-min grain), `usage_interface_15m` (headline totals), `connections` (raw window), `apps`.
- **API:** `GET /usage/summary` (total/attributed/unattributed), `GET /apps` (ranking), `GET /apps/{id}/usage`.

### Build order
1. Compaction job raw → rollups at 7-day seam (PLAN P4.1 — pull into this feature's critical path)
2. `GET /usage/summary` with top apps (P4.2)
3. `GET /apps` ranking + `GET /apps/{id}/usage` (P5.1/P5.3 backend halves)
4. Dashboard headline + Top apps UI (P4.6 partial) + stat cards
5. Week/month boundaries computed at query time in the configured **timezone**; "this month" = billing cycle; document in `docs/api-contract.md` (D4a).

### Edge cases & validation
- **Soft-deleted apps** still aggregate (rollups FK-join `apps` — soft delete keeps history intact, DATABASE.md §2.2).
- **Clock changes:** 15-minute UTC buckets are immune to DST; day boundaries *displayed* in local time per configured timezone — edge-day totals may look ±1 h; caption the timezone (D4a).
- **Month boundary:** "this month" = current billing cycle at query time (not UTC calendar month); verify a mid-cycle query never includes last cycle's tail (D4a).
- **Renamed/replaced binaries** (hash change): new `apps` row via `exe_path` uniqueness; old history stays attributed to the old row — never rewrite history.
- **Validation:** `top_limit` ≤ 10 (API.md §2.5); granularity enum enforced.

### States
- **Loading:** headline skeletons + chart placeholder per DESIGN.md §8.3.
- **Empty:** zeroed totals only after first real sample, with `"state":"collecting"`; before that, empty state.
- **Error:** chart-area error banner + retry; totals cards degrade to "—" not fake zeros.

### Acceptance criteria
- [ ] Daily totals equal kernel interface counters by construction; ≥95% of bytes attributed; unattributed row shows gap (accuracy harness)
- [x] Top apps order matches summary; click-through routes to F4
- [x] Compaction preserves totals (raw + rollup = constant across the seam)
- [ ] Soft-deleted app's history still resolves name/icon
- [ ] Timezone caption correct on day/month boundaries

---

## 3. F3 — Dashboard

### What & story
Landing page: usage timeline (hour/day toggle), top-talker bars, headline totals, per-interface breakdown; metered interfaces labeled (PRD §6.1 #3).
Covers: *"As a user, I want a dashboard chart of usage over time, so that I can understand patterns at a glance"* (PRD §7).

### Screens & components
- **Dashboard** (§8.3) only. Components: headline cards, ECharts wrappers (theme from §3.3 — download=series 1, upload=series 2), Day|Hour tabs (§7.13), interfaces card, empty state, chart error banner, chart data-table fallback (§9.4).

### Data contracts
- **DB:** `usage_rollup_15m`/`usage_interface_15m`, `connections` (raw window), `interfaces` (`is_excluded`).
- **API:** `GET /usage/summary` (total/attributed/unattributed), `GET /usage/timeline`, `GET /usage/interfaces`, `WS usage.totals`.

### Build order
1. `GET /usage/timeline` with raw+rollup union (PLAN P4.3, flag F5)
2. `GET /usage/interfaces` (P4.4)
3. ECharts token theme wrapper + data-table fallback (P4.5, P9.6 pulled early for charts here)
4. Dashboard screen assembly + WS totals (P4.6–P4.7)
5. Empty state (P4.8)

### Edge cases & validation
- **Raw/rollup seam (7 days):** one continuous series; double-counting across the seam is a P0 bug (harness checks constant totals).
- **Excluded interfaces** (`lo`): never counted anywhere (U3 filter).
- **Metered interface:** distinct badge — informational in MVP, no blocking behavior (PRD non-goal).
- **Chart data volume:** decimate > ~2k points (ARCHITECTURE.md §9); toggle re-query, never fake interpolation.
- **Validation:** `granularity=hour|day` only; `since<until` enforced else 400.

### States
- Loading: skeleton headline + chart placeholder. Empty: "Waiting for first data" + live hint. Error: chart-area error banner + retry; totals degrade to "—".

### Acceptance criteria
- [ ] Timeline continuous across 7-day seam; totals constant (P10.1 harness later proves)
- [ ] Series order/colors exactly DESIGN.md §3.3; theme switch re-renders
- [ ] Day/Hour toggle works from tabs; charts resize on container
- [ ] `lo` excluded everywhere; metered badge shows
- [ ] Chart has accessible data-table fallback; reduced-motion respected

---

## 4. F4 — App Detail Page

### What & story
Per-app drill-down: usage over time, open connections, recent history, endpoint summary, first/last seen (PRD §6.1 #4).
Covers: *"As a user, I want to click any app and see its detail page"* + developer-persona investigation stories (PRD §7).

### Screens & components
- **App detail** (§8.5): icon + name + headline usage; tabs Overview / Connections / Endpoints (§7.13); usage chart; connections table (this app); endpoint ranked list (mono counts); attribution-confidence badge; icon fallback (§6.2).

### Data contracts
- **DB:** `apps`, `connections` (by `app_id`), `usage_rollup_*`.
- **API:** `GET /apps/{id}`, `GET /apps/{id}/usage`, `GET /apps/{id}/endpoints`, `GET /connections?app_id=` (reuse F5's endpoint).

### Build order
1. `GET /apps/{id}` + icon resource endpoint (PLAN P5.2)
2. `GET /apps/{id}/usage` + `/endpoints` (P5.3)
3. App detail screen: three tabs wired (P5.4)
4. Click-through from Dashboard top-apps + Live rows (P5.5)
5. Confidence display for heuristic/unknown attribution (P5.6)

### Edge cases & validation
- **Soft-deleted app:** page still renders from history (`include_deleted` semantics); caption "No longer active — last seen <date>".
- **Pruned window:** Connections tab shows only retained rows; caption states retention window, never pretends data was never there.
- **No icon:** fallback container — never a broken image.
- **Endpoint list cap:** top 100 with "and N more" note (API cap), no pagination needed in MVP.
- **Validation:** app id must exist → 404 with honest message; endpoints limit ≤ 100.

### States
Loading: per-tab skeletons. Empty (brand-new app with no bytes yet): tab-specific empty states. Error: tab-area banner + retry.

### Acceptance criteria
- [ ] All three tabs populated from real data; tabs keyboard-navigable (§9.3)
- [ ] Click-through works from Dashboard and Live
- [ ] Soft-deleted/pruned cases render honestly with captions
- [x] Unknown-icon fallback shown; confidence badge on non-exact attribution
- [ ] Endpoint ranking matches manual SQL spot-check

---

## 5. F5 — Local History

### What & story
SQLite-backed connection log with 90-day default retention, auto-pruning, basic search by app name (PRD §6.1 #5). Pro multi-filter/export arrive in F9 — this feature ships the free tier.
Covers: *"As a user, I want a 90-day searchable history"* (PRD §7).

### Screens & components
- **History screen** (§8.6): app search + date-range inputs (Pro chips locked — F9), results table (virtualized), row-count/retention caption, Export menu (Pro, visible but gated).
- Components: tables (§7.6), search input (§7.2), pagination via infinite keyset scroll, empty/error/loading states.

### Data contracts
- **DB:** `connections` + endpoint-day rollups (`usage_endpoint_day` for history beyond raw window); retention pruner (P7.1) enforces the tier policy.
- **API:** `GET /connections` (keyset pagination, basic filters free; advanced set 403 until F9), `GET /connections/{id}`. **Beyond the raw window, rows are aggregate rows from `usage_endpoint_day` (flagged `"kind": "aggregate"`), not connection episodes.**

### Build order
1. Migration 0003 + index set (PLAN P3.1)
2. `GET /connections` keyset pagination + basic filters (P3.2)
3. Pro-gate scaffold returning 403 (P3.3 — locked off until F9)
4. `GET /connections/{id}` + pruned-window 404 wording (P3.4)
5. History screen (P3.5)
6. Restart-persistence test + `apps` soft delete (P3.6–P3.7)
7. Pruner wired to tier (P7.1 — lands with F8, referenced here)

### Edge cases & validation
- **Cursor invalidation:** pruning removes rows under a held cursor → next page may skip; that's acceptable and documented (keyset, not offset); UI never errors on `has_more=false`.
- **Pruning mid-session:** rows visible at load may vanish on re-query — caption states retention, no client-side resurrection.
- **App search escaping:** substring search escapes SQL LIKE wildcards (`%`/`_`).
- **Huge results:** hard `limit ≤ 500`; client fetches pages on scroll; no unbounded queries.
- **Validation:** sort whitelist (`first_seen_at`, `last_seen_at`, `bytes_up`, `bytes_down`, `remote_ip`); combined filters → 403 (pre-F9); `since ≤ until`.

### States
Loading: skeleton rows, header visible. Empty: "No connections match" (search) vs "No history yet" (fresh db) — distinct empty states. Error: banner + retry.

### Acceptance criteria
- [ ] Restart keeps history (integration test)
- [ ] Keyset pagination walks 10k+ rows without dup/skip within a stable snapshot
- [ ] Free tier: app-name search works; any advanced filter → 403 path rendered as Pro lock
- [ ] Pruned row → 404 with "outside retention window" message
- [ ] Mono right-aligned numerals everywhere; virtualized > 500 rows

---

## 6. F6 — Daemon + Install Experience

### What & story
systemd service install/enable via pkexec (once), health checks, clear degraded states with self-help text (PRD §6.1 #6).
Covers: *"As a user, I want clear status when the daemon isn't running or lacks permissions"* + install-once story (PRD §7).

### Screens & components
- **Onboarding** (§8.2): 3 cards (what/where/how), Enable button, failure card.
- **Daemon-stopped banner** (§7.9, non-dismissible) + **Diagnostics modal** (§8.9).
- Components: `StatusChip` (§7.5), banners, primary/ghost buttons.

### Data contracts
- **DB:** `daemon_events` (evidence), `settings`.
- **API:** `GET /health` (no auth), `GET /status`, `GET /diagnostics`, `GET /events`; hardened unit (PRD §5).

### Build order
1. Unix-socket listener + peer-credential UID allowlist; allowed-UID list written during onboarding pkexec (PLAN P1.1) + `/health`, `/status` (P1.2)
2. Hardened systemd unit (P8.1): sandboxing, capability bounding
3. Packages + post-install registering unit disabled-by-default (P8.2–P8.4)
4. Onboarding flow: pkexec enable (registers UID allowlist, one prompt) → health checks → 3 cards → Dashboard (P8.5)
5. Failure paths: missing BTF, denied pkexec, unwritable data dir (P8.6)
6. Diagnostics endpoint + modal + copy-report (P9.1–P9.3)

### Edge cases & validation
- **pkexec cancelled/denied:** failure card with reason + retry; no silent retry loop.
- **Kernel without BTF:** exact error class (`EBPF_UNSUPPORTED_KERNEL`) → guidance link + "Run without monitoring" path.
- **UID not in allowlist:** socket refused → "Stopped" chip, not crash; re-run onboarding to add UID.
- **systemd absent/non-Linux:** pre-flight check in installer; onboarding explains.
- **Socket reconnect:** UI auto-recovers ≤ 5 s after daemon restart; UID list persists (no token rotation).
- **Validation:** onboarding blocks navigation until enabled *or* user explicitly picks degraded/demo path.

### States
Onboarding: idle → loading (pkexec) → success auto-route → failure card. Banner states per DESIGN.md §7.9. Diagnostics: checks ✓/✗/— (mono result text).

### Acceptance criteria
- [ ] Fresh VM: install → onboarding → live data < 3 min (all 3 package paths)
- [ ] Each degraded state (no daemon, no eBPF, permission denied) has a tested UX path
- [ ] Banner is non-dismissible while daemon is down; actions recover to Monitoring
- [ ] Diagnostics shows checks + journal excerpt + copy-report
- [ ] `daemon_events` rows land for every failure class (evidence exists)

---

## 7. F7 — Tray Integration

### What & story
Tray icon with today's total usage tooltip; menu: open dashboard, pause UI notifications, quit UI — quitting the UI never stops the daemon (PRD §6.1 #7).
Covers: *"As a privacy-conscious user, I want a tray icon showing today's usage"* + quit-safety story (PRD §7; Flow 6).

### Screens & components
- No in-app screen. Tray menu per REAPM.md §8 pattern (safe separation): Open dashboard / Pause notifications / Quit.
- Components: Tauri tray (`app/src-tauri/src/tray.rs`), confirm dialog (§7.4) for quit.

### Data contracts
- **DB:** none directly — tray reads a cached today-total updated from `WS usage.totals` (UI process only).
- **API:** `WS usage.totals`; daemon link for live-state tooltip caption.

### Build order
1. Tray icon + static menu (PLAN P7.5)
2. Tooltip binding to cached today-total (≤ 1 msg/s channel)
3. Quit flow: confirm dialog with "UI closes; monitoring continues" + optional "Also stop monitoring" (pkexec)

### Edge cases & validation
- **DE without tray support** (GNOME default): graceful degradation — app still works; hint in Settings; document GNOME extension requirement (PRD R5).
- **Daemon down:** tooltip shows "Stopped" state text, never a stale number.
- **Quit vs kill:** window close (X) minimizes to tray by default (setting in F8); real quit only via menu/confirm.
- **Validation:** "Also stop monitoring" requires pkexec; cancelling leaves daemon running.

### States
Tooltip: number (mono) or honest state text. Menu items disabled when daemon unreachable (with tooltip explaining).

### Acceptance criteria
- [ ] Tray quit keeps daemon running (verified via `/health` after quit)
- [ ] "Also stop monitoring" stops via pkexec; cancel leaves it running
- [ ] Tooltip updates ≤ 5 s; shows state text when degraded
- [ ] GNOME no-tray case degrades gracefully with docs hint

---

## 8. F8 — Settings (Lean)

### What & story
Retention length, theme, start-minimized, tray behavior, metered-interface labeling, daemon start-on-boot (PRD §6.1 #8).
Covers settings-related stories (PRD §7) and OQ5 (metered detection: manual labeling in MVP).

### Screens & components
- **Settings screen** (§8.7): sections General / Data / Interfaces / Daemon / License / About.
- Components: inputs (§7.2), switches (§7.13), destructive confirm (§7.4) for clear-data, undo toasts (§7.8).

### Data contracts
- **DB:** `settings` (key/value mirror), `interfaces` (metered/excluded).
- **API:** `GET /settings`, `PUT /settings/{key}` (typed, bounded; free retention ceiling → 403), `PATCH /interfaces/{id}`.

### Build order
1. `GET/PUT /settings` with type/bounds (PLAN P7.2)
2. Settings screen sections (P7.3)
3. Interfaces labeling (P7.4)
4. Theme switching persisted + chart re-theme (P7.6)
5. `metered_policy`/`start_on_boot` plumbing (P7.7)
6. Clear-data destructive flow + undo toast

### Edge cases & validation
- **Concurrent edits** (two windows): last-write-wins per key; UI refreshes on WS `status`/settings change — no merge UI.
- **Out-of-range/wrong type:** 400 with field name → inline error (§7.2); value never half-applied (single-row tx).
- **Retention > 90 on free tier:** 403 → upgrade modal (§8.8), input reverts.
- **Timezone change:** day/week/month boundaries recomputed at query time; charts re-query (D4a).
- **Billing-cycle day change:** "this month" boundary shifts; caption reflects new cycle (D4a).
- **Theme change:** applies live, ECharts re-registers theme (no restart).
- **Clear data:** destructive confirm naming the consequence ("Deletes 90 days of local history"); undo toast only until pruner next pass? No — clear-data is immediate+irreversible → confirm only, no undo toast (§7.4).

### States
Every control: default/hover/focus/disabled per §7.2/§7.13. Save: immediate (no Save button) — failed saves revert visually with toast error.

### Acceptance criteria
- [ ] All settings persist across restart (settings table is source of truth)
- [ ] Invalid values rejected with inline errors; nothing half-applied
- [ ] Free tier can't set retention > 90 (403 → upgrade modal)
- [ ] Theme applies live and survives restart
- [ ] Metered labeling reflects on Dashboard card immediately

---

## 9. F9 — Freemium Mechanics

### What & story
Free tier as shipped in F1–F5; Pro license key unlocks advanced history search (multi-filter), CSV/JSON export, retention beyond 90 days; clearly gated in-UI with upgrade prompt (PRD §6.1 #9).
Covers Pro stories (PRD §7) + Flow 4.

### Screens & components
- **Upgrade modal** (§8.8): entitlements list, key input, Activate / Buy buttons.
- **History** (§8.6): Pro filter chips unlock; Export menu (CSV/JSON → native save dialog); lock badges (§7.7).
- Components: secondary accent reserved for Pro (DESIGN.md §3.4), inline input errors, toasts.

### Data contracts
- **DB:** `license` (singleton; fingerprint only, never the raw key).
- **API:** `GET /license` (entitlements), `POST /license/activate`, `DELETE /license`; gating server-side on `GET /connections` (advanced filters), `POST /exports` (+ job/content endpoints), `PUT /settings/{key}` (retention ceiling); export rate limit 5/min (API.md §1.5).

### Build order
1. Migration 0004 + `daemon_events` (PLAN P6.1)
2. Offline verifier: signature check, no network (P6.2)
3. License endpoints + L1 entitlements shape (P6.3)
4. Server-side gating wired on C2/E1–E3/T2 (P6.4)
5. Export engine: job → content CSV/JSON (P6.5)
6. Upgrade modal + instant unlock via WS `status` (P6.6)
7. History Pro locks + export menu (P6.7–P6.8)

### Edge cases & validation
- **Key validation:** offline-only (embedded public key); expired/malformed/tampered → 403 `LICENSE_INVALID` with distinct messages.
- **Double activation:** 409 `CONFLICT` (no-op offer in message).
- **Deactivate mid-export:** started jobs complete (grace window), new ones 403; UI toast explains.
- **Export size/timeouts:** 500-row preview estimate at creation; content endpoint streamed; job expiry 10 min → 404 with message.
- **DB corruption of license row:** verifier falls back to free honestly (no crash, `daemon_events` warn).
- **Buy button:** opens user-triggered external URL (the app itself never fetches it — zero-outbound preserved).

### States
Upgrade modal: idle → validating (loading state on Activate) → success (modal closes, WS unlocks UI) / error (inline). History locks: disabled chips + tooltips pre-Pro.

### Acceptance criteria
- [ ] Activate with valid offline-signed key unlocks instantly, no restart (Flow 4)
- [ ] All gated paths return 403 server-side for free tier (filters, export, retention) — verified with raw curl, not just UI
- [ ] Export produces correct CSV/JSON of filtered rows; rate limit enforced
- [ ] Deactivate returns every lock; started exports complete
- [ ] No network call during validation (Live view proves zero-outbound)

---

## 10. F10 — Updates & Packaging

### What & story
AUR package, .deb + apt repo, .rpm + dnf repo; signed updates; clean uninstall with opt-in data retention (PRD §6.1 #10).
Covers: *"As a user, I want to install once with a package manager and never think about permissions again"* (PRD §7).
**Design note:** updates flow through package managers (repos), not an in-app updater — this preserves the zero-outbound promise (deliberate contrast to the REAPM.md reference app's in-app updater).

### Screens & components
- **Onboarding** (§8.2) reuse for post-update first-run notes; **About section** (F8): version (mono), licenses, no-telemetry statement.
- No other screens; `schema_migrations` visible only via Diagnostics version line (API.md flag F4).

### Data contracts
- **DB:** `schema_migrations` (forward-only migrations run at daemon boot on upgrade).
- **API:** `GET /health` version line; diagnostics DB size/version.

### Build order
1. Hardened unit + packages (mostly F6): .deb, .rpm, AUR `-git` (PLAN P8.1–P8.4)
2. Repo signing + release workflow (P8.8)
3. Stable repos + AUR `-`/`-bin` (P11.1)
4. Upgrade path: package update → migration run → health re-check (P10.4 validates)
5. Uninstall completeness test (P8.7)

### Edge cases & validation
- **Version skew UI vs daemon:** UI shows "Updating…" honest state if `/health` version < UI expectation; refuse destructive ops on mismatch.
- **Signature failure:** package manager blocks install (upstream behavior); repo key rotation documented.
- **Schema migration failure at boot:** daemon refuses start with distinct exit code → degraded UX, never silent partial schema (DATABASE.md §8).
- **Uninstall:** service files 100% removed; data dir removal is opt-in (PRD metric).
- **Validation:** release checklist = PRD §11 DoD; packaging matrix in PLAN.md §1.

### States
N/A for UI beyond About/onboarding captions; failure states live in F6's paths.

### Acceptance criteria
- [ ] Clean-VM install via each channel → working app (all 3 distro families)
- [ ] Package upgrade runs migrations forward-only; history intact
- [ ] Uninstall removes all service files; data opt-in honored
- [ ] Repos + artifacts signed; CI matrix green on all 3
- [ ] No in-app network fetch during updates (zero-outbound intact)

---

## 11. Build Order & Confirmation Gates

**Recommended sequence:** F1 → F2 → F3 → F4 → F5 → F6 → F7 → F8 → F9 → F10.

- F2/F3 share the compaction work; F3 reuses F2's summary backend — build in that order to avoid rework.
- F9 depends on F5's 403 scaffold and F8's settings ceiling — both land before it.
- F10 is last because it packages everything; its F6-dependent parts arrive early.

**After each feature:** tick its PLAN.md/CHECKLIST.md boxes, bump the status table in §0, tag if a phase completed — then **stop and wait for the user's confirmation**. Do not start the next feature until confirmed.

---

*End of FEATURES.md. Update §0 status table and PLAN/CHECKLIST after every completed feature.*
