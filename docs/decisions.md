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

## D8 — IPv6 Flow Accounting in Live & History (2026-11-21)

**Status:** Reverted: IPv6 bytes are counted per app; per-connection IPv6 is not tracked.
**Next step:** IPv6 header-parsing spike using `bpf_skb_load_bytes` (see `docs/ROADMAP.md` §2 / `docs/inbox/ROADMAP-PLAN-2026-11-23.md` §2).

**Context:** Commit `2249397` removed the IPv6 flow reader because the Linux kernel
`cgroup_skb` verifier rejected reading `local_ip6` and `remote_ip6` context offsets
with `Permission denied (os error 13)`. IPv6 traffic is aggregated in per-app totals
via the `BYTES` cgroup map, but individual IPv6 connections are not tracked in Live
or History.

### Superseded design (Task 5)

*(Retained for reference; reverted in commit 2249397)*

1. **BPF `Flow` struct:** Expanded IP fields from 32-bit (`u32`) to 128-bit (`[u32; 4]`):
   ```rust
   #[repr(C)]
   #[derive(Clone, Copy)]
   pub struct Flow {
       pub cgroup_id: u64,
       pub local_ip: [u32; 4],
       pub remote_ip: [u32; 4],
       pub local_port: u32,
       pub remote_port: u32,
       pub protocol: u32,
       pub _pad: u32,
       pub bytes_up: u64,
       pub bytes_down: u64,
       pub packets_up: u32,
       pub packets_down: u32,
       pub family: u32, // AF_INET = 2, AF_INET6 = 10
       pub _pad2: u32,
   }
   ```
   IPv4 addresses are stored in `local_ip[0]` / `remote_ip[0]` with `family = 2`; IPv6 addresses store all 4 words with `family = 10`.
2. **Memory & Map Impact:**
   - Struct size increases from 64 bytes to 88 bytes (+24 bytes per entry).
   - In `crates/netwatchd/src/ebpf.rs`, matching `#[repr(C)] Flow` with size assertion: `assert_eq!(std::mem::size_of::<Flow>(), 88)`.
   - `FLOWS` map overhead in kernel: At `MAX_CGROUPS` = 1024, additional memory is `1024 * 24 B = ~24.5 KiB` (negligible).
3. **Database Schema:**
   - `connections` table schema (`DATABASE.md` §3) uses `local_ip TEXT` and `remote_ip TEXT`. Standard IPv6 strings (up to 39 characters) fit without table alteration or migration.
4. **UI Display & Sorting:**
   - Formatted via standard canonical RFC 5952 representation (`std::net::Ipv6Addr::to_string()`).
   - Monospace tabular presentation with middle truncation or tooltip if width is constrained.

---

## D9 — Upload Accounting & GSO Segments (2026-11-23)

**Status:** PROPOSED (owner approval pending; spike result pending)

**Context:** The eBPF spike report (`docs/spike-ebpf-report.md`) identified an upload
byte discrepancy: NIC interface statistics count ~42 bytes per packet more than
`cgroup_skb` egress (tx attributed was measured at 57–66% of interface counters).

**Hypothesis:** `cgroup_skb` egress observes large GSO (Generic Segmentation Offload)
skbs before the network stack splits them into wire packets. The NIC counts Ethernet,
IP, and TCP headers once per wire packet, whereas NWP currently counts them once per
skb. If true, `skb->gso_segs` indicates the number of wire packets, and:
`attributed_bytes = skb_len + (gso_segs * header_size)`
should reconcile the difference.

**Decision options:**
1. Spike `gso_segs` and length logging during 60 s iperf3 upload against
   `/sys/class/net/<if>/statistics/tx_bytes`.
2. If `gso_segs` correction brings tx attribution within 5% of NIC, apply the
   daemon correction.
3. If not, openly document and display the known upload attribution gap in the UI
   and rewrite the sum-check gate accordingly.

---

## D10 — Scope Boundary: v1.0 Has No New Data Source (2026-11-23)

**Status:** PROPOSED (owner approval pending)

**Context:** The feature catalogue contains over 300 line items across 23 sections,
many requiring new privileged collectors or operating system hooks (DNS observation,
active probes, process focus tracking, wireless nl80211, etc.).

**Proposal:**
1. v1.0 contains no new data source beyond what NWP currently collects (interface
   counters via sysfs, per-cgroup/per-socket byte accounting via Aya eBPF,
   process/desktop metadata via sysfs/procfs).
2. v0.10 "Plan" adds user-value features on existing data: budgets, forecasting,
   desktop notifications, leaderboard, heatmaps, CLI (`nwp status`, `nwp top`,
   `nwp export`), and compact tray monitor.
3. v0.11 "Explore" adds deep analysis on existing data: connection explorer (IPv4;
   IPv6 if spike passes), app profile pages (sessions as cgroup lifetime),
   comparison studio, basic network identity, smart history archive, and portable
   backup/restore.
4. Hardening (v1.0.0-rc.1 and v1.0.0) focuses strictly on packaging (.deb, .rpm,
   PKGBUILD), VM matrix verification across 4 distros, and a 14-day soak test.

---

## D11 — Release Sequence for Post-1.0 Releases (2026-11-23)

**Status:** PROPOSED (owner approval pending)

**Context:** Staging features by data dependency (what the collector can truthfully
know) prevents rework and premature complexity.

**Proposed release order:**
- **v1.1 "Insight":** Local baselines from existing rollups + logind idle/lock state
  via D-Bus (anomaly detection, idle investigator, activity scores, fingerprint metrics).
- **v1.2 "Destinations":** DNS observation (domain tracking, app-to-domain mapping,
  offline geo lookup).
- **v1.3 "Network Health":** Active probes + Wi-Fi/link stats (latency, jitter,
  packet loss, gateway health, full network profiles).
- **v1.4 "Time Machine":** Unified event store (timeline, correlation, replay,
  PDF/scheduled reporting).
- **v1.5 "Desktop":** Desktop adapters (layer-shell widgets, panel extensions, tray).
- **v2.0 "Control and Extend":** Optional separate components for traffic control
  and plugins (subject to explicit owner approval).

---

## D12 — Reshaping Catalogue Items for Truthful Presentation (2026-11-23)

**Status:** PROPOSED (owner approval pending)

**Context:** Certain vision catalogue items assume capabilities that cannot be
accurately or privately delivered on Linux (especially Wayland).

**Proposed reshaped specifications:**
1. **Foreground vs. Background Usage:** Replace window focus tracking (unsupported/blocked
   across Wayland compositors) with logind `IdleHint` and `LockedHint` D-Bus state
   ("active", "idle", "locked"). Window focus is optionally tracked on X11 only, clearly labelled.
2. **Application Sessions:** Define and label a session as the lifetime of an
   application's cgroup (cgroup creation to destruction), not window or GUI sessions.
3. **Traffic Classification:** Label classifications (e.g. streaming, background sync)
   as "estimated", displaying the heuristic rule and confidence score.
4. **Domain Intelligence:** Track domains exclusively from observed DNS responses;
   DoH and DoT traffic are labelled "unknown" rather than guessed.
5. **Geographic Lookup:** Optional, completely offline IP database bundled or
   user-provided; off by default to protect the zero-outbound privacy promise.

---

## D13 — Separation of Traffic Controls and Plugins (2026-11-23)

**Status:** PROPOSED (owner approval pending)

**Context:** Section 12 (application network blocking / firewall rules) and Section 23
(plugin system / custom scripts) transform NWP from a read-only passive monitor into
an active controller with severe privilege escalation and network cutoff risks.

**Proposal:**
1. Traffic controls (app blocking, scheduled restrictions, pause mode) and plugins are
   deferred to v2.0 or later and excluded from v1.0 scope.
2. If implemented, app blocking must be built as a separate standalone privileged
   component (for example `netwatch-guard`) with its own dedicated polkit action and
   nftables cgroup filtering, isolated from `netwatchd` so a monitoring failure can
   never break network connectivity.
3. Plugins must never execute inside the root daemon context.
4. Owner must explicitly decide whether traffic control belongs in the Net Watch Pro
   product vision at all.

---

## D14 — Pricing and Key Expiry Recheck Precondition (2026-11-23)

**Status:** PROPOSED (owner approval pending)

**Context:** The current decision keeps Net Watch Pro completely free until other
ecosystem applications reach beta. Audit gap G8 identified that offline Pro license keys
are verified at activation but expiry is not rechecked periodically on daemon boot.

**Proposal:**
1. Net Watch Pro remains 100% free through v1.0. All v1.0 features (including export
   and filters) remain accessible.
2. Fixing gap G8 (periodic license expiration recheck in `netwatchd`) is logged as a
   mandatory gate before any commercial or paid license key tier is introduced.

