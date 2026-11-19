# docs/decisions.md — Owner Decisions (Plan Fix Effort)

**Version:** 1.0 · **Date:** 2026-11-15  
**Source:** `PLAN-FIX-INSTRUCTIONS.md` Part 2 (D1–D7). One section per decision, dated, with the chosen option and rationale.

---

## D1 — UI transport: Unix domain socket + peer-credential UID allowlist

**Chosen:** option (a) — Unix domain socket owned by root, peer-credential (`SO_PEERCRED`) check against a root-owned allowed-UID list written during the one-time onboarding pkexec step.

**Rationale:** One admin prompt at enable-time, none per launch; removes the loopback TCP surface entirely (no CORS/Origin/CSRF/rate-limit machinery needed for a single trusted local client); honors PRD §8 Flow 1 "install once and never think about permissions again".

**Scope change:** Replaces `ARCHITECTURE.md §5.3` loopback TCP + token auth contract and `PRD §5` IPC contract. Token-helper/Polkit removal follows after this decision (F-06 mitigation).

---

## D2 — Long-term history: daily endpoint rollups with per-app/day cap

**Chosen:** option (a) — add `usage_endpoint_day` table keyed `(app_id, bucket_start, remote_ip, remote_port, protocol)` with columns `bytes_up`, `bytes_down`, `connection_count`, `first_seen_at`, `last_seen_at`; keep a cardinality cap (top N endpoints per app per day by bytes, rest folded into an `other` row).

**Rationale:** Satisfies PRD §6.1 #5/#9 promise of host/port/searchable history beyond the raw window without keeping raw rows forever (which F-02 shows is impossible with the current 7-day compaction).

---

## D3 — Headline total usage: interface kernel counters + Unattributed row

**Chosen:** option (a) — headline "total usage" comes from per-interface kernel counters (`/sys/class/net/*/statistics` or netlink stats); per-app numbers are attributions of that total; the gap is shown as an explicit "Unattributed / system" row.

**Rationale:** Matches what an ISP bills (PRD Persona 2), is honest about header/ACK/retransmit/kernel-handled traffic gaps (F-03), and avoids the unattainable "sum of per-app == interface counters" claim.

---

## D4 — Rollup base grain: 15 minutes + timezone/billing-cycle at query time

**Chosen:** option (a) — `usage_rollup_15m` base table; day/week/month boundaries computed at query time in the configured IANA timezone; add `timezone` (IANA, default system) and `billing_cycle_day` (1–31, default 1) settings.

**Rationale:** UTC hour/day buckets cannot produce exact local "today" in half-hour/45-minute offset zones (e.g. Asia/Kolkata +5:30, Nepal +5:45); metered users reason in billing cycles (F-04).

---

## D5 — App identity key: cgroup-derived identity first, exe_path fallback

**Chosen:** option (a) — capture process identity in-kernel at exec/connect time (sched_process_exec tracepoint or equivalent), resolve cgroup path via cgroupfs; `/proc` reads become a fallback; `apps` gets `app_key TEXT UNIQUE` + `identity_kind` (`cgroup_app|flatpak|snap|exe`); `exe_path` is non-unique display/provenance data.

**Rationale:** Avoids the `/proc/<pid>/exe` vs. bounded-capability contradiction (F-05); handles Flatpak/Snap/script interpreters where `exe_path` collapses multiple apps into one row.

---

## D6 — MVP positioning: data-usage and metered-connection tracking

**Chosen:** option (a) — the differentiator is data-usage tracking for metered-connection users; budgets/alerts (`PRD §6.2 #1`) move into MVP scope; Persona 2 becomes co-primary with Persona 1.

**Rationale:** F-07 positioning finding — "no product occupies the monitor-only niche" is no longer accurate; the real gap is usable per-app data-usage accounting with history. Competitor analysis to be summarized in PRD §2.1 (own words, cited links).

---

## D7 — Available time: 40+ h/week (full-time)

**Chosen:** 40+ hours per week.

**Rationale:** The current PLAN timeline (13 weeks to v1.0.0, single full-time builder) remains valid; effort estimates stay in hours, labeled estimates.

---

## OQ4 — Demo mode after onboarding failure (decided in P8.6, 2026-11-19)

**Chosen:** ship a minimal read-only demo with seeded in-memory sample data.
When the one-time pkexec enable fails, the onboarding error card offers "See a
demo instead"; a persistent "Demo mode" badge in the topbar lets the user exit
back to onboarding. The demo feeds the existing Dashboard and History stores
from `app/src/lib/fixtures/demo.ts` — no daemon, no files beyond a local
preference flag, and no network traffic.

**Rationale:** PRD Flow 1 step 7 promises an explorable UI when monitoring
cannot be enabled. A small local fixture set satisfies that promise without a
mock socket server or sample-data files on disk, preserving the zero-outbound
contract. A separate "Run without monitoring" full-mock backend was considered
and cut as unnecessary scope.

---

## L-OQ1 — Last-hour history resolution (decided 2026-11-19)

**Chosen:** keep a small **1-minute in-memory history** in the daemon for the
last-hour view; the 15-minute rollups remain the persistent source for longer
ranges. The in-memory buffer is lost on daemon restart and the UI must say so
instead of pretending continuity.

**Rationale:** four 15-minute points are too coarse for "how fast was I
downloading an hour ago"; a 60-point buffer at one-minute resolution covers
the whole last hour (a newer 1 Hz sample in the current minute replaces the
previous point so the view stays fresh).

## L-OQ2 — Live speed unit default (decided 2026-11-19)

**Chosen:** keep **`mbps`** as the default for `speed_unit`; `mbytes` remains
selectable. No change to the plan.

## L-OQ3 — Partial attribution label (decided 2026-11-19)

**Chosen:** if the accepted spike shows only TCP can be attributed reliably,
ship per-app live speeds with an explicit **"TCP only"** label and a visible
Unattributed remainder rather than waiting for full UDP coverage.

**Rationale:** an honest partial number with a confidence label is more useful
than no per-app speed, provided the sum check still includes the remainder.

## L-OQ4 — Wi-Fi signal and link speed (decided 2026-11-19)

**Chosen:** schedule Wi-Fi signal strength and negotiated link speed
**immediately after Phase A** (before the eBPF spike). They were previously
dropped from this plan; this decision overrides that. Values come from local
sysfs (`/sys/class/net/<iface>/wireless/link`, `/sys/class/net/<iface>/speed`),
never from a network call. Missing values are shown as "—", never invented.
**[HYPOTHESIS]** `wireless/link` is read as dBm per this decision; verify on
the target hardware, since some kernels expose a 0–100 link-quality value
there instead (see ARCHITECTURE.md §5.1b).
