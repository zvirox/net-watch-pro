# Net Watch Pro — Live Throughput Implementation Plan

Nov 19, 2026 · @Prakash

## 1. Goal and scope

Net Watch Pro must show how fast this PC is moving data right now, and how fast each app is. If Motrix downloads at 10 Mbps and uploads at 1 Mbps, the dashboard shows that for the whole PC, and the Apps list shows Motrix with the same two numbers.

**Requirements**

- **PC live speed:** current download and upload in Mbps, refreshed every second, with a 60-second sparkline. A setting switches the unit between Mbps (default) and MB/s, because download managers such as Motrix report MB/s.
- **Per-app live speed:** current download and upload for each app in the Apps list, grouped by app identity, so the row says "Motrix", not "aria2c".
- **Usage over time:** dashboard presets for the last hour, 24 hours, 7 days, 30 days and the billing cycle.
- **Honest status:** the UI never says monitoring is active when no collector is running.

**Non-goals**

- No internet speed test. It needs outbound traffic, which breaks the zero-outbound promise.
- No Wi-Fi signal strength or negotiated link speed in this plan. They were dropped from the request and can follow later.
- No firewall or blocking features.

**Assumption:** in the Motrix example, 10 Mbps is download and 1 Mbps is upload, as for a normal download.

## 2. Current state

The dashboard is empty for three separate reasons, and only the first is fixed. Findings come from reading `main` at commit `14060db` (19 Nov 2026, 15:53); the app itself was not run.

| # | Cause | Evidence in the code | Status |
| --- | --- | --- | --- |
| 1 | The daemon could not see host network interfaces | The systemd unit had `PrivateNetwork=yes`, so `/sys/class/net` showed only `lo`, which the sampler skips | Fixed in `14060db`. The installed unit must be reloaded and the daemon restarted |
| 2 | No per-app data source exists | `main.rs` creates the live-event sender as `_events` and never uses it. There is no eBPF loader or collector. P2.1 and P2.2 are blocked | Open. This is Phases B and C |
| 3 | No rate calculation | The interface sampler runs every 15 s into 15-minute buckets, and the live totals stream reads only today's total from the database | Open. This is Phase A |

**Other problems found**

- `/status` always returns daemon "monitoring" and eBPF "attached" (`main.rs`), even with no collector.
- `CHECKLIST.md` says 1 Hz live deltas come from the `/sys` sampler. No code does that, so the note is wrong. P2.3 is also ticked, but nothing feeds attribution.
- The eBPF spike produced no measurements. It used C and libbpf instead of Aya, and ran in what looks like a restricted environment. That cause is a hypothesis, not a measurement.
- Dashboard range presets are not confirmed. The timeline API supports hour and day buckets, but I did not verify the UI controls.

**After the unit fix:** reload and restart with `sudo systemctl daemon-reload && sudo systemctl restart netwatchd`. Wait about 30 seconds, because the first sample is only a baseline. Then check `sudo sqlite3 /var/lib/netwatch-pro/netwatch.db 'select * from usage_interface_15m'` for rows.

## 3. Target architecture

One rate engine turns byte counters into speeds for both the PC total and each app, and the UI reads only the daemon's streams.

&#91;embedded content: data flow · 2 sources, 1 rate engine, 3 outputs\]

Phase A builds the left path, from interface counters to the PC speed card. Phase C adds the dashed per-app path. The existing 15-second sampler keeps writing history in the background, so usage over time does not depend on the new streams.

## 4. Phase A: live PC speed and honest status

Phase A needs no eBPF and gives visible data first: a 1-second rate sampler, a `net.rates` stream, a "Network now" card, and a truthful `/status`. Estimated effort is 12 to 16 hours.

| ID | Task | Where (verify paths before editing) | Done when |
| --- | --- | --- | --- |
| A1 | Update docs first (section 10) | `PRD.md`, `API.md`, `ARCHITECTURE.md`, `docs/api-contract.md`, `docs/env-vars.md` | Docs merged with version bumps before any code |
| A2 | Rate math as pure code: bytes delta over measured elapsed time, times 8, then smoothing | new `crates/netwatch-core/src/rates.rs` | Unit tests cover counter reset, counter decrease, interface appearing and disappearing, and a skipped tick |
| A3 | 1 Hz rate task reading `/sys/class/net/*/statistics`, reusing the existing interface exclusion list | new `crates/netwatchd/src/rates.rs`, wired in `main.rs` | The task survives read errors and reports a degraded event, as the 15 s sampler does |
| A4 | `net.rates` stream channel with a snapshot on subscribe, plus the shared payload types | `crates/netwatch-common/src/lib.rs`, `crates/netwatch-api/src/stream.rs` | Subscribing delivers one frame per second, and an unsubscribed client gets none |
| A5 | "Network now" card with download and upload numbers and a 60-second sparkline | `app/src/lib/stores/` (new rates store), `app/src/lib/screens/dashboard/Dashboard.svelte` | Loading, idle (0 bps), stale (no frame for 3 s) and error states render per DESIGN.md |
| A6 | `speed_unit` setting, `mbps` or `mbytes`, default `mbps` | `DATABASE.md` settings seed, a new forward-only migration, `crates/netwatch-api/src/settings.rs`, settings screen | Changing it updates the card without a restart |
| A7 | Honest `/status`: report the real collector state, "no collector" until one attaches | `crates/netwatchd/src/main.rs`, `crates/netwatch-common/src/lib.rs` (`EbpfState`), UI status chip | With no collector the chip is not green and the Live and Apps screens say why |
| A8 | Correct `CHECKLIST.md`: untick P2.3, fix the "1 Hz live deltas" note, record this plan's tasks | `CHECKLIST.md`, `PLAN.md` | Tracker matches the code |

**Rate rules**

- Use the real elapsed time between reads (a monotonic clock), not a nominal 1 second, so a delayed tick does not inflate the rate.
- Units are decimal: 1 Mbps = 1,000,000 bits per second, 1 MB/s = 1,000,000 bytes per second. Label them in the UI.
- Smooth with an exponential moving average of about 3 seconds so the number does not flicker. Keep the constant in one place.
- The PC total is the sum over non-excluded interfaces, with the same tunnel double-counting rule the docs already define for interface totals.
- Add `EbpfState` values only if none fits "no collector". Check the enum first, and update `API.md` S2 if it changes.

**Acceptance checks**

1. Run a download (for example `curl` of a large file or `iperf3`). The card matches the tool's own reported rate within about 10% after a 3-second warm-up.
2. With the network idle, the card reads about 0 and does not show noise.
3. Restart the daemon: the card shows loading, then values, with no crash and no negative number.
4. Daemon stopped: the existing daemon-stopped banner appears and the card shows its stale state.

## 5. Phase B: eBPF spike on your own PC, with Motrix

Phase B decides how per-app bytes are counted, and it must run on a real machine with real root, not in the coding agent's sandbox. Time box: 25 hours, as in the original plan. The earlier spike produced no measurements, so every kernel claim in the docs stays `[HYPOTHESIS]` until this phase reports.

**Why the earlier spike failed.** Its report shows `/sys/fs/cgroup` mounted as bpffs and CAP\_BPF missing from the effective set under sudo, which looks like a restricted environment. This is a hypothesis. It also tried "all attach types 0 to 15", but cgroup\_skb has only ingress and egress and needs an open cgroup file descriptor, so that test method was flawed.

**Setup**

- Rebuild the spike in Aya (the mandated stack), not C and libbpf. The scaffold is at `spikes/ebpf-aya/` (Design 1 cgroup_skb byte counters, Design 2 kprobe identity, Design 3 hybrid) with the owner runbook in its README. Record the exact toolchain: nightly Rust, `bpf-linker`, versions and build time.
- Run on your PC as root. Run a second kernel in a VM later (Ubuntu LTS or Fedora) for the kernel-variance check.
- Compare Design 1 (cgroup\_skb counters keyed by socket cookie) with Design 3 (hybrid), as in `PLAN-FIX-INSTRUCTIONS.md` Part 3.

**Workloads, Motrix first**

| Workload | What it proves |
| --- | --- |
| Motrix HTTP download | TCP attribution, and whether the `aria2c` child process lands in the same cgroup or app scope as Motrix |
| Motrix torrent download | UDP traffic (uTP, DHT) is counted, since `sock_diag` would miss it |
| Motrix started from the desktop launcher, then from a terminal | Whether the cgroup identity changes with how the app is started |
| `curl` large download, `iperf3` TCP and UDP | Accuracy against a known rate |
| 5,000 short `curl` runs | Short-lived process handling and map churn |
| Python script making requests | Scripts not collapsing into one "python3" app |

**Questions to answer, with measurements**

1. Per-app bytes versus interface counters: what share is attributed, and what makes up the gap?
2. Is the per-second per-app rate stable enough to display (jitter, lag behind the true rate)?
3. CPU and memory use at idle, at 50 and 500 connections, and at full-speed download.
4. Which capabilities are really needed to load, attach and run? Test the bounded set `CAP_BPF`, `CAP_PERFMON`, `CAP_NET_ADMIN`.
5. Does cgroup identity group Motrix and `aria2c` into one app, and can a display name come from the `.desktop` file?
6. Does Secure Boot or kernel lockdown block anything?

**Go / no-go**

- **Go** if, on the `curl` and `iperf3` runs, attributed bytes are within about 5% of the true traffic, attributed plus unattributed matches the interface counters within about 2%, and overhead stays within the daemon's CPU and memory budget.
- **No-go or re-scope** if UDP or short-lived processes cannot be attributed reliably. Then the UI must show a confidence label per row instead of exact numbers. Per owner decision L-OQ3, this is **not** a reason to withhold per-app speeds: Phase C ships them with a visible **"TCP only"** label and an Unattributed remainder, provided the sum check still holds.
- Write the result in `docs/spike-ebpf-report.md` and re-tag each claim `[VERIFIED]` or `[REFUTED]`. Do not start Phase C before the owner accepts the report.

## 6. Phase C: collector and per-app live speed

Phase C turns the spike's chosen design into the real collector and puts a live download and upload rate next to every app, grouped so that Motrix shows as "Motrix". It starts only after the owner accepts the Phase B report. Estimated effort is 30 to 40 hours, and the range depends on what the spike finds.

| ID | Task | Done when |
| --- | --- | --- |
| C1 | eBPF programs and loader in the `netwatch-ebpf` crate (separate nightly build), attaching at daemon start | The daemon attaches with the bounded capability set, and a failed attach becomes a degraded state, not a crash |
| C2 | Kernel-to-daemon path: per-socket byte counters read each second, with drop counters for the ring buffer and map pressure | Dropped-event counts are visible in Diagnostics |
| C3 | Identity resolution: cgroup-first (systemd app scope, Flatpak, Snap), exec-time capture, `/proc` as fallback, as decided in D5a | A Motrix download is attributed to one app, not to `aria2c` and Motrix separately |
| C4 | Display names: resolve an app's name and icon from its `.desktop` entry, so the row reads "Motrix" | Unknown apps fall back to the executable name with a lower confidence label |
| C5 | Per-second per-app rates: bytes delta over elapsed time, smoothed the same way as the PC total | A new `live.apps` stream carries one entry per active app |
| C6 | Feed attributed bytes into the existing episode store and rollups so history and the "attributed" total fill in | The dashboard's Unattributed row shrinks to the real gap |
| C7 | Apps list shows live download and upload per app, sorted by current total speed, with idle apps below | The list updates every second and does not reorder jitterily (hold order for about 2 seconds) |
| C8 | Live Connections screen shows real rows | The screen's empty state appears only when nothing is connected |
| C9 | Degraded path: `NETWATCH_DISABLE_EBPF` and failed attach give an honest UI | The status chip, Live, Apps and Dashboard all explain the missing data |

**Grouping rule.** An app's identity key is its cgroup-based app scope. Processes in the same scope, such as Motrix and its `aria2c` child, add into one row. When two apps share a scope (everything started from one terminal), the row shows the process name and a lower confidence label instead of merging unrelated programs. This rule is unverified until the spike tests Motrix launched both ways.

**Sum check.** On every refresh the sum of per-app rates plus the unattributed remainder must equal the PC total from Phase A. Show the remainder as an "Unattributed / system" row instead of hiding it.

## 7. Phase D: usage over time and the data cycle

Phase D adds range presets so the same dashboard answers "how much did I use in the last hour, today, this week, this month and this billing cycle", for the whole PC and per app. Much of the backend already exists (15-minute rollups, a timezone-aware calendar, billing-cycle and budget logic), so this phase is mostly UI and small API additions. Estimated effort is 8 to 12 hours, and it can run in parallel with Phase B.

| ID | Task | Done when |
| --- | --- | --- |
| D1 | Range selector: 1 h, 24 h, 7 d, 30 d, billing cycle, remembered per user | Each preset changes the headline totals, the timeline and the top-apps list together |
| D2 | Totals per range from the rollup tables, using the configured timezone and `billing_cycle_day` | A test with timezone `Asia/Kolkata` shows "today" resetting at local midnight, not at 05:30 |
| D3 | Timeline buckets per range: 15 minutes for 1 h and 24 h, hours for 7 d, days for 30 d and the cycle | A bar chart with a table fallback, in the DESIGN.md chart theme |
| D4 | Per-app usage for the selected range (extends `/apps` ranking and `/apps/{id}/usage`) | The Apps list and an app's detail page respect the selected range |
| D5 | Billing-cycle card: bytes used, days left, and the existing budget state | Matches the cycle start day setting, including a month shorter than the cycle day |

**Check before building.** Read the existing usage summary, timeline and apps endpoints. Add a `range` parameter only where it is missing, and update `API.md` and `docs/api-contract.md` in the same change.

**Open point for the owner.** The last-hour view has only four 15-minute points. A small in-memory one-minute history in the daemon would make it smoother, at the cost of losing it on restart. See the open questions in section 9.

## 8. API and UI contract

Two new stream channels carry the live numbers, one setting controls units, and three screens change. The payloads below are proposals: copy the frame envelope and field style from the existing `usage.totals` channel, and record the final shape in `API.md` and `docs/api-contract.md` before coding.

**Stream `net.rates`** (Phase A, one frame per second while subscribed):

```json
{ "ts": 1763548800,
  "down_bps": 10000000,
  "up_bps": 1000000,
  "interfaces": [ { "name": "wlan0", "down_bps": 10000000, "up_bps": 1000000 } ] }
```

**Stream `live.apps`** (Phase C, one frame per second while subscribed):

```json
{ "ts": 1763548800,
  "apps": [ { "app_id": 12, "name": "Motrix", "down_bps": 10000000,
              "up_bps": 1000000, "confidence": "exact" } ],
  "unattributed": { "down_bps": 120000, "up_bps": 30000 } }
```

Rates are bits per second as integers; the UI converts to Mbps or MB/s. Only apps with non-zero traffic in the last few seconds are listed.

**Settings:** `speed_unit` (`mbps` default, or `mbytes`). It needs a seed row via a forward-only migration, and a `DATABASE.md` update first.

**Screens**

| Screen | Change | States to handle |
| --- | --- | --- |
| Dashboard | New "Network now" card at the top: download and upload numbers with units, 60-second sparkline. Range selector (Phase D) above the totals, timeline and top apps | Loading, idle (0), stale (no frame for 3 s), daemon stopped, no collector |
| Apps | Columns for live download and live upload, plus total for the selected range. Default sort by current speed. Unattributed row at the bottom | Empty, loading, no collector (explains why per-app speeds are missing), low-confidence label |
| Settings | Speed unit selector | Saved, error |
| Status chip | Real collector state instead of a fixed "attached" | Monitoring, degraded (no collector), stopped |

**Rules**

- The UI reads only through the daemon's socket API, as the existing rules require. It never reads `/sys` or `/proc` itself.
- Each frame carries a timestamp. If the newest frame is older than 3 seconds, show a stale state instead of the last number.
- No new colours or components. Reuse the DESIGN.md tokens and existing card, table and chart components.

## 9. Testing, acceptance and risks

The feature is done when a Motrix download shows up live, at the right speed, in both the PC total and the Apps list. These checks decide that, and they must be run, not assumed.

**Acceptance tests**

| # | Scenario | Pass condition |
| --- | --- | --- |
| 1 | Large `curl` or `iperf3` download | PC card within about 10% of the tool's rate after a 3-second warm-up |
| 2 | Network idle | PC card about 0, with no negative or flickering values |
| 3 | Motrix HTTP download at a known speed | Apps list shows one row, "Motrix", with download near the Motrix figure and upload near its upload figure |
| 4 | Motrix torrent download | UDP traffic is included, so the row is not far below the PC total |
| 5 | Sum check | Sum of app rates plus Unattributed equals the PC total on each frame |
| 6 | Daemon restart | Card and list recover within 5 seconds, no crash, no negative numbers |
| 7 | No collector (attach fails or `NETWATCH_DISABLE_EBPF`) | Status chip is not green, Apps explains why, PC card still works |
| 8 | Timezone `Asia/Kolkata` | "Today" resets at local midnight |
| 9 | Zero outbound traffic | The app makes no network call, checked while the Live view is open |

**Unit tests**

- Rate math: counter reset, decrease, wrap, interface added or removed, a late tick, smoothing step response.
- Grouping: same-scope processes merge, different-scope processes do not.
- Stream: frame cadence, snapshot on subscribe, backpressure with a slow client.

**CI.** Process-level tests use a fake sysfs tree. The real eBPF path runs on a self-hosted or VM runner on a schedule, because containers share the host kernel.

**Risks**

| Risk | Effect | Mitigation |
| --- | --- | --- |
| Spike shows UDP or short-lived processes cannot be attributed | Per-app speeds are low or wrong for torrents | Confidence label per row, and show the Unattributed gap instead of hiding it |
| Motrix and `aria2c` land in different scopes | Two rows instead of one | Merge by parent process or desktop entry as a fallback, and test both launch methods |
| 1-second polling of many sockets costs CPU | Misses the daemon CPU budget | Read kernel maps in one batch per second and measure in the spike |
| Smoothed values lag the true speed | Numbers look slow to react | Keep the smoothing constant small and tune against a real download |
| Hardened unit blocks something the collector needs | Attach fails on some distros | Test the bounded capability set early and document each addition |

**Open questions for the owner — answered 2026-11-19 (see `docs/decisions.md` L-OQ1–L-OQ4)**

1. Should the last-hour view use a small in-memory one-minute history (smoother, lost on restart) or only the 15-minute rollups? → **One-minute in-memory history** (60 points cover the last hour; labelled lost-on-restart).
2. Should units default to Mbps or MB/s? This plan defaults to Mbps and offers MB/s in settings. → **Keep Mbps** as the default; MB/s stays selectable.
3. If the spike shows only TCP can be counted reliably, ship per-app speeds with a visible "TCP only" label, or wait for full coverage? → **Ship with a "TCP only" label** and a visible Unattributed remainder; do not wait for UDP.
4. Wi-Fi signal strength and link speed were dropped from this plan. Do you want them scheduled after Phase A? → **Yes, immediately after Phase A** (local sysfs only, "—" when missing).

## 10. Docs first, agent rules and ready-to-paste prompt

Update the docs before writing code, because adding streams and a setting is a scope change under `AGENTS.md` section 3. This plan is the owner's approval for that change. To give it to the coding agent, export this doc as Markdown and commit it as `docs/LIVE-THROUGHPUT-PLAN.md`.

**Docs to update first, each with a version bump and a history line**

| Doc | Change |
| --- | --- |
| `PRD.md` | Add live PC speed and per-app live speed to the MVP features and Definition of Done |
| `ARCHITECTURE.md` | Add the 1 Hz rate task and the `live.apps` path to the daemon data flow and task list |
| `API.md` and `docs/api-contract.md` | Add `net.rates` and `live.apps`, the `/status` collector states, and any `range` parameters |
| `DATABASE.md` | Add the `speed_unit` setting seed and its forward-only migration |
| `DESIGN.md` | Add the Network now card, the Apps speed columns and the stale and no-collector states |
| `FEATURES.md`, `TESTING.md`, `PLAN.md`, `CHECKLIST.md` | Add the tasks and tests from sections 4 to 9, and fix the CHECKLIST claims listed in A8 |
| `AGENTS.md`, `AUTH_SECURITY.md`, `API.md` section 1.1, `README.md`, `SECURITY.md` | Earlier audit found old "token-authenticated loopback API" wording. Re-check and fix any that remains |

**Rules for the agent**

- Follow `AGENTS.md`: plan first, small commits, Conventional Commits, no new dependencies without asking.
- Tag unmeasured kernel claims `[HYPOTHESIS]`, and never claim a check you did not run.
- Do Phase A and stop for approval. Do not start Phase C before the spike report is accepted.
- Phase B must be run by the owner on the real PC, or the agent must say it cannot.
- This plan did not re-audit the socket authentication code. Do not assume the UID allowlist is enforced.

**Prompt to paste**

```
Read AGENTS.md, docs/decisions.md, docs/spike-ebpf-report.md and docs/LIVE-THROUGHPUT-PLAN.md.
Implement the plan in order. Requirement: the dashboard shows LIVE network speed in Mbps
(MB/s selectable) for this PC and per app (for example Motrix 10 Mbps down, 1 Mbps up).
No internet speed test, and the app makes zero outbound connections.

Step 1: update the docs listed in section 10 first (PRD, ARCHITECTURE, API, api-contract,
DATABASE, DESIGN, FEATURES, TESTING, PLAN, CHECKLIST), with version bumps. The owner
approves this scope change.
Step 2: Phase A (tasks A2-A8). Then run the checks you can, list the ones you cannot run,
summarize, and STOP for my approval.
Step 3: only after I accept docs/spike-ebpf-report.md, do Phase C. Phase B (the eBPF spike in
Aya, with Motrix as workload) is run by me on my own PC with real root. Write the spike code
and the README so I can run it, and record my results in the report.
Step 4: Phase D range presets (can run in parallel with Phase B).

Rules: small commits, no new dependencies without asking, mark unmeasured kernel claims
[HYPOTHESIS], never claim verification you did not run.
```
