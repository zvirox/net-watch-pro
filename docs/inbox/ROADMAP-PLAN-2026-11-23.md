<!-- Source plan merged into docs/ROADMAP.md -->

# Net Watch Pro — Fix Plan, Vision Review & Roadmap to v1.0

Nov 22, 2026 · @Prakash

## 1. Verdict

**The vision is strong, but it is four products wearing one name, so it must be built in layers, not in the order of its 23 sections.** The target below is v1.0 stable in late April 2027, with the intelligence features following in v1.1 to v2.0.

- **Where NWP is today:** live PC throughput and per-app rows work on your CachyOS host (Motrix \~87–100 Mbps, rollups persisted). The current plan is not closed: see gaps G1–G8 in section 2.
- **Why the vision is strong:** a per-app, offline, explainable network ledger for Linux is a real gap. Most Linux tools show either live totals (bandwidth meters) or packets (Wireshark-style); few give per-app history, budgets and explanations together.
- **Why it needs discipline:** the catalogue has well over 300 line items, with the same idea listed up to three times (launch correlation, fingerprinting, idle traffic, comparison, time machine). A solo developer with coding agents should ship about 25% of it by v1.0 and the rest in releases that each prove one new data source.
- **Rule that orders everything:** build in order of *data dependency* (what the collector can truthfully know), then user value, then effort. Features that need data NWP does not collect yet (DNS, sessions, idle state, network identity) come after the release that adds that data.
- **Needs your decision:** blocking apps with a firewall (section 12 of the vision) and a plugin system change NWP from a monitor into a controller. I recommend v2.0 at the earliest, as separate optional components. See section 11.

Estimates in this doc are planning figures for one developer plus agents, based on how fast Phases A–D and the security work landed in the last week. They are not commitments; each release is gated by acceptance tests, not by date.

## 2. Stage 0: close the current plan (Nov 23 to Dec 6)

**Stage 0 turns the repo's claims into things that are true, then ships v0.9.2-beta.5.** Nothing here adds a feature. Each gap comes from the audit in the previous message; severity is for a public beta.

| ID | Gap | Fix | Done when | Effort |
| --- | --- | --- | --- | --- |
| G1 | CHANGELOG v0.9.2-beta.4 and decisions D8 claim IPv6 live flows; commit 2249397 removed them | Rewrite both to "IPv6 bytes counted per app; per-connection IPv6 not tracked". Add a `Known limits` line in README | No doc claims more than the code does; grep for "IPv6" finds no contradiction | 0.5 day |
| G2 | CHECKLIST stale: L-C1–L-C8 unticked, Phase S S.2–S.5 say BLOCKED, tag cut before DoD | Tick what host evidence proves, link the evidence, remove BLOCKED notes. Rule: no tag before the Phase DoD line is ticked | Checklist matches `docs/spike-ebpf-report.md` line by line | 0.5 day |
| G3 | `accuracy.py`, `perf_budget.py`, `soak.py` run with eBPF off on seeded data | New `tests/host/attribution.py` (needs root, eBPF on): iperf3 and curl of known size, compare per-app bytes and NIC counters. Keep the old tests, rename them `synthetic_*` | Report prints attributed % and unattributed %; CPU and RSS measured with the collector running | 3 days |
| G4 | Upload undercount: NIC counts about 42 B/packet more than `cgroup_skb` (tx 57–66% attributed) | Spike first (below), then either correct in the daemon or show it openly in the UI. Record as decision D9 | tx attributed is within 5% of NIC on iperf3, or the UI states the known gap and the sum-check gate is rewritten | 2–3 days |
| G5 | Plan §9 tests not run on host: #4 torrent/UDP, #6 restart recovery, #9 zero outbound with collector | Run on the CachyOS host, paste output into the spike report | Three results recorded as pass or fail with numbers | 1 day |
| G6 | VM matrix (Q6) *(RETIRED)* | Replaced by CI clean-container smoke tests for locked `.deb`, `.rpm`, `.pkg.tar.zst` (`zvirox/release-workflows`) | Container smoke tests green | Retired |
| G7 | Polkit helper ignores `usermod` and drop-in failures | Report each failed step with a clear message and non-zero exit; keep idempotence | Helper test with a read-only `/etc/systemd` shows an error, not silence | 0.5 day |
| G8 | Key expiry never rechecked after activation | Not needed while NWP is free. Log in `docs/decisions.md` as a precondition for any paid tier | Entry exists | 15 min |

**Spike for G4 [HYPOTHESIS].** `cgroup_skb` egress sees a large GSO skb before the NIC splits it into wire packets, so the NIC adds Ethernet, IP and TCP headers once per wire packet while NWP counts them once per skb. If true, `skb->gso_segs` gives the number of wire packets and `bytes + gso_segs × header_size` should approach NIC bytes. Test: log `gso_segs` and `len` for 60 s of iperf3 upload and compare with `/sys/class/net/<if>/statistics/tx_bytes`. If it fits, correction is cheap; if not, show the gap honestly.

**Spike for IPv6 flows [HYPOTHESIS].** The verifier rejected reading `local_ip6` and `remote_ip6` from the `cgroup_skb` context. Parsing the IPv6 header with `bpf_skb_load_bytes` (L3 starts at offset 0 in `cgroup_skb`) may pass. Timebox: 2 days in Stage 1. If it fails, keep the honest "IPv4 per connection" label and move on.

**Release v0.9.2-beta.5 (target Dec 6).** Includes G1, G2, G5, G7, G8 and the G3 harness. G4 and G6 may land in beta.6 if the spike runs long.

## 3. Vision review

**The vision holds together around one idea: explainable, offline, per-app network history.** That idea is what to protect; everything that does not serve it waits.

**What is strong**

- Per-app attribution plus history plus budgets in one local tool. Your Motrix test already proves the hardest part.
- "Explainable" and "evidence-based" findings (sections 2, 14, 21 of the catalogue). They turn a number into a reason and avoid scary unexplained risk scores.
- Offline and privacy-first. This is a real differentiator and fits the AGPL repo.
- Honest limits (encrypted traffic, attribution confidence). Users of monitoring tools notice when a tool overclaims.

**What is risky**

- **Breadth.** The catalogue has well over 300 items. At the pace of the last weeks, building all of it would take years, and each feature adds tests, docs and bugs.
- **Repetition.** The same capability appears in several sections, so counting sections overstates the work:
  - Launch correlation: sections 1 and 20.
  - Fingerprinting: sections 1, 2 and 21.
  - Idle traffic: sections 3 and 19.
  - Time-based browsing: sections 5 and 15.
  - Comparison: sections 1, 16 and 18.
  - Budgets: sections 7 and 8.

  Build each capability once, then expose it in several places.
- **Controller creep.** Sections 12 (blocking, firewall) and 23 (plugins, custom scripts) turn a read-only monitor with one privileged daemon into something that can change the system. That raises the security bar sharply and conflicts with the current threat model in `docs/security-review.md`.
- **Platform limits.** Floating widgets, always-on-top and click-through are restricted on Wayland. Foreground-vs-background app detection is unreliable there because Wayland does not tell other programs which window has focus.

**What the collector can and cannot truthfully know today**

| Capability | Known today | Needs new data source | Honest limit |
| --- | --- | --- | --- |
| Bytes per app, per interface, per cgroup | Yes (IPv4 and IPv6) | None | Upload undercount (G4) |
| Per-connection bytes | IPv4 only | IPv6 header parsing spike | IPv6 flows missing |
| Sessions and app launches | No | cgroup create/destroy events or process exec events | Sessions are cgroup lifetimes, not window sessions |
| Foreground vs background | No | Focus tracking | Not reliable on Wayland; use "user active / idle / locked" from logind instead |
| Idle and screen-locked state | No | logind `IdleHint` and `LockedHint` over D-Bus | Idle hint depends on the desktop |
| Domains and DNS correlation | No | Observe DNS replies (resolver or UDP 53 packets) | DoH and DoT hide names; label "unknown" |
| Network identity (SSID, VPN, profile) | No | NetworkManager D-Bus or netlink | Differs per distro and network manager |
| Latency, jitter, loss | No | Active probes (ICMP/UDP) from a helper | Needs unprivileged ping sockets or CAP_NET_RAW |
| Wi-Fi signal and link speed | No | `/proc/net/wireless`, nl80211 | Wired and some drivers report nothing |
| Traffic type (streaming, syncing) | No | Heuristics on rate, direction, duration | Label as "estimated" with a confidence |
| Destination geography | No | Offline IP database | Optional; accuracy varies by database |

So several headline features (Domain Intelligence, Idle Investigator, Network Profiles, Performance Intelligence) are not features yet: each is a new data source with its own failure modes, and each should get its own release.

## 4. Build in layers, not in catalogue order

**Five layers carry all 23 sections, and a feature may only be built once the layer under it is true.** This is why Domain Intelligence cannot start before DNS observation exists, and why heatmaps and leaderboards can ship early: they only need layers NWP already has.

[embedded content: data layers L0 to L4 · release per layer]

Building by layer also stops duplicates: the baseline engine (L3, v1.1) is built once and serves behavior change, anomalies, idle baselines and fingerprints from sections 2, 3, 19 and 21.

## 5. Priority rubric and release gates

**Every feature gets a priority from four questions, asked in this order.** A feature that fails question 1 is not scheduled, whatever its value.

1. **Is the data true?** Can the collector or a system source supply it with a known accuracy today? If not, it waits for the release that adds the source.
2. **Does it serve the core promise?** Per-app, offline, explainable network history. Features that only decorate wait.
3. **Value to a typical user** (casual user checking data, then power user, then security-minded user).
4. **Effort and risk** (new privileged code, new kernel hooks and new UI surfaces cost most).

| Priority | Meaning | Example features | Lands in |
| --- | --- | --- | --- |
| P0 | Needed to finish the current plan | Gaps G1–G8 | v0.9.2-beta.5 and beta.6 |
| P1 | Defines a trustworthy v1.0 monitor | Budgets, forecasting, notifications, connection explorer, heatmaps, leaderboard, backup, monitoring health, CLI | v0.10 and v0.11 |
| P2 | Makes NWP stand out; each needs a new data source | Anomalies, idle investigator, domains, performance, profiles, time machine | v1.1 to v1.4 |
| P3 | Reach and polish | Desktop widgets, panel integrations, custom dashboards | v1.5 and v2.0 |
| P4 | Changes the product's trust model | App blocking, plugins, custom scripts | v2.0 or later, only with your approval |

**Gates, applied to every release.** No tag until all of these are ticked in `CHECKLIST.md` with evidence:

- Acceptance tests for each new feature pass on real hardware with eBPF on (not on seeded data only).
- Every user-visible number states how it was measured, and every known limit is written next to the feature, in the UI and in the docs.
- `cargo test`, UI tests, lint and the security checks pass. New privileged code gets a security-review entry first.
- CHANGELOG lists only what the code does. A claim without a test is marked [HYPOTHESIS].
- Scope changes are written in `docs/decisions.md` first and approved by you (AGENTS.md §3), then built.

**Cut rule.** If a release slips more than two weeks, drop its lowest-priority items instead of moving the date. Dates in section 6 are targets for one developer plus agents.

## 6. Timeline to v1.0 stable

**Five months of gated releases take NWP from today's beta to v1.0 stable on Apr 25, 2027.** Each bar ends at a tag; a tag happens only when its acceptance tests pass, so a late bar moves the ones after it.

[embedded content: roadmap to v1.0 · 6 phases, 1 launch]

The first 14 days are repair work, the next two releases add what makes the data useful (budgets, forecasts, explorer), and the last 7 weeks are hardening. Releases after v1.0 are in section 8.

## 7. Scope per release up to v1.0

**v1.0 contains everything a user needs to trust NWP as a daily monitor, and none of the features that need a new data source.** About a quarter of the catalogue ships by then; the rest is already ordered in section 8.

### v0.9.3 "Trust" (target Dec 20, 2026)

- Finish G4 (upload accounting), locked `nfpm` container smoke verification (G6 VM test retired), and the IPv6 flow spike result.
- Monitoring Health and Self-Diagnostics (catalogue 23): collector status, attach mode, permission problems, gaps, in plain language.
- Data Attribution Transparency (23): a page explaining how bytes are measured and where totals can differ from the router or ISP.
- Data Integrity Validation (22): a `check` command that verifies rollups against totals.
- **Accept when:** a user who breaks the collector (no permission, no eBPF) sees what is wrong and the fix, on all four test distros.

### v0.10 "Plan" (target Jan 24, 2027)

- Budgets (7, 8): billing cycle already exists; add quota (total, upload, download), per-app and per-interface budgets, warning, critical and maximum thresholds, remaining and daily allowance.
- Forecasting (7): month-end, weekly, budget exhaustion date, overage cost, FUP tracking from user-entered plan data. Use simple, documented math (average of recent days with weekday weighting) and show the inputs.
- Notifications (11, 23): desktop notifications, quiet hours, deduplication, projected overage alerts.
- Leaderboard (18): rankings by total, upload, download and peak; period selector.
- Heatmaps (17): daily calendar, hourly, weekday pattern, built from existing rollups.
- CLI and JSON export (13, 23): `nwp status`, `nwp top`, `nwp export`; aggregated historical export.
- Compact monitor mode and tray indicator (11): small window and a StatusNotifier tray icon. No floating overlay yet.
- **Accept when:** a budget set on Day 1 warns at the right percentage on a seeded cycle test, and the forecast error on a 7-day replay is shown in the UI.

### v0.11 "Explore" (target Mar 7, 2027)

- Connection Explorer (4): search and filter by app, IP, port, protocol, state, duration, volume and time; LAN or WAN; loopback filter; listening socket inventory; service names for ports; IPv6 per-connection if the spike passed, otherwise labelled IPv4 only.
- App Network Profile page (1, 2): total, daily average, peak, connection count, network-active time, trend, upload/download ratio. Session means one cgroup lifetime and is labelled that way.
- Comparison (16): apps, periods, interfaces, upload vs download.
- Network identity basics (10): interface and network switching history, new network detection, a user-set trust level (including "metered"), no automatic profiles yet.
- Smart History Archive (22): configurable retention per data type, downsampling that preserves totals, archive size view, availability indicator per period.
- Backup and Restore (13, 23): portable backup with validation before restore.
- **Accept when:** restoring a backup on a clean machine reproduces totals to the byte, and the connection table filters return correct rows on a seeded set of 100k connections within the performance budget.

### v1.0.0-rc.1 (target Apr 4) and v1.0.0 (target Apr 25, 2027)

- Feature freeze at rc.1. Only fixes after that.
- Packaging locked exclusively to `nfpm` (`packaging/nfpm.yaml`): Debian/Ubuntu (`.deb`), Fedora (`.rpm`), and Arch (`.pkg.tar.zst`) with the eBPF object included, install/uninstall verified in clean CI containers (`ubuntu:22.04`, `ubuntu:24.04`, `fedora:latest`, `archlinux:latest`). *(AUR, Flatpak, AppImage, tarballs, and VM tests permanently retired.)*
- Refresh `docs/security-review.md`, run maintainer-script hardening checks, run a 14-day soak on your own machine with eBPF on, record CPU and RSS.
- Docs: user guide, known limits page, upgrade and migration notes from beta schema.
- **v1.0 accept when:** the soak shows no data loss across daemon restarts, CPU and memory stay inside the perf budget measured with eBPF on, zero open P0 or P1 bugs, and every CHECKLIST line for Phases A–D and 10 is ticked with evidence.

## 8. After v1.0: one new data source per release

**Each release after v1.0 adds exactly one new kind of data and the features that depend on it.** That keeps every release testable and keeps the collector stable. Windows are planning targets and widen after v1.0 because each source brings new failure modes.

| Release | Window | Theme and new data source | Main features (catalogue section) |
| --- | --- | --- | --- |
| v1.1 "Insight" | Jun 2027 | Local baselines from existing rollups, plus logind idle and lock state | Spike, unexpected upload, new app, large transfer and unusual timing alerts (3); idle investigator (19); insights feed with evidence and acknowledge (14); explainable activity score (2); traffic fingerprint metrics: volume, timing, direction ratio (21); estimated traffic classes (1) |
| v1.2 "Destinations" | Aug 2027 | DNS observation | Domain tracking and IP mapping (6); app-to-domain; first and last seen; destination ranking (18); categories and custom categories; attribution confidence; optional offline geo lookup; domain explorer; DNS correlation (5) |
| v1.3 "Network Health" | Oct 2027 | Active probes, Wi-Fi and link stats | Latency, jitter, loss, gateway, DNS speed, outages, Wi-Fi signal, link speed, interface errors and drops, quality score (9); full network profiles: per-profile budgets, alerts, targets, privacy (10) |
| v1.4 "Time Machine" | Dec 2027 | A unified event store | Event timeline, correlation, investigation view, time machine navigation and replay (5, 15, 20); annotations, bookmarks, tags, saved views (23); PDF and scheduled reports, chart export (13) |
| v1.5 "Desktop" | Q1 2028 | Desktop adapters (DBus and compositor APIs) | Floating widget and per-app overlay where the compositor allows it; KDE panel widget; GNOME indicator or extension; COSMIC; hotkeys; widget presets (11) |
| v2.0 "Control and Extend" | Q2 2028 or later | Separate privileged components; only with your approval | Custom dashboards and advanced notification rules (23); integration hooks; plugin system; optional per-app blocking and scheduled restrictions (12) |

**Why this order**

- Alerts before domains: anomaly detection needs only bytes, which NWP already has, and it is the feature users feel first.
- Domains before performance probes: DNS observation adds no active traffic; probes add network traffic and a new privilege question.
- Time Machine late: it needs events from sources v1.1–v1.3 to be worth replaying, and it is mostly UI on top of them.
- Desktop integration late: each desktop (KDE, GNOME, COSMIC, Wayland vs X11) is its own support burden; build it behind a stable local API and a `Desktop Adapter` boundary (catalogue 11).
- Control last: blocking needs firewall rules written by a privileged component. Keep it out of the monitoring daemon so a monitoring bug can never cut someone's network.

**Free vs paid.** You decided to keep NWP free until your other apps reach beta. Nothing here assumes a paid tier; if you later split features, the natural candidates are v1.1 and later intelligence features, and the expiry-recheck fix (G8) must land before any paid key ships.

## 9. Where each vision section lands

**Every one of the 23 sections has a home; eleven of them are split across releases because their parts need different data.** Use this table when the agent asks "which release does X belong to?".

| # | Vision section | Ships in | Notes |
| --- | --- | --- | --- |
| 1 | Deep Application Intelligence | v0.11 (profile page, sessions as cgroup lifetime), v1.1 (idle consumption, classification, fingerprint history) | Foreground vs background becomes "active, idle, locked" (see section 10) |
| 2 | Application Behavior Score | v0.11 (metrics), v1.1 (change detection, explainable score) | Score stays explainable and never uses security labels |
| 3 | Anomaly Detection and Alerts | v1.1 | Spike alerts are simple threshold rules in v0.10 notifications |
| 4 | Network Forensic Explorer | v0.11 (IPv4, IPv6 if spike passes), v1.2 (hostnames) | Connection history depends on retention settings |
| 5 | Connection Timeline and Investigation | v1.2 (DNS correlation), v1.4 (timeline, investigation) | |
| 6 | Domain and Destination Intelligence | v1.2 | Encrypted-traffic limits stated in the UI |
| 7 | Forecasting and Usage Planning | v0.10 | Application forecasting after the app baselines exist (v1.1) |
| 8 | Smart Budget System | v0.10 | Billing cycle already done |
| 9 | Network Performance Intelligence | v1.3 | Interface errors and link speed can come earlier if cheap |
| 10 | Network Profiles and Context | v0.11 (identity, trust level), v1.3 (full profiles) | |
| 11 | Interface and System Integration | v0.10 (tray, compact mode, notifications), v1.5 (widgets, desktops) | Desktop Adapter architecture designed in v0.10 |
| 12 | Advanced Traffic Controls | v2.0, only with approval | Separate component |
| 13 | Reporting, Automation, Export | v0.10 (CSV, JSON, aggregated), v0.11 (backup), v1.4 (PDF, scheduled, chart export) | |
| 14 | Network Intelligence Engine | v1.1 | One shared baseline engine serves sections 2, 3, 19, 21 |
| 15 | Network Time Machine | v1.4 | |
| 16 | Traffic Comparison Studio | v0.11 (apps, periods, interfaces, direction), v1.3 (profiles), v1.4 (multi-entity) | |
| 17 | Usage Heatmap | v0.10 | Pure rollup views |
| 18 | Usage Leaderboard | v0.10 (apps), v1.2 (destinations) | |
| 19 | Idle Activity Investigator | v1.1 | Needs logind idle and lock state |
| 20 | Event Correlation Engine | v1.4 | Needs events from v1.1–v1.3 |
| 21 | Application Traffic Fingerprinting | v1.1 | Shown as metrics, not labels |
| 22 | Smart History Archive | v0.9.3 (integrity), v0.11 (retention, downsampling, availability) | |
| 23 | Additional Ultimate-Level | Split: health, diagnostics, CLI, API, privacy and offline in v0.9.3–v0.10; backup in v0.11; annotations, tags, saved views in v1.4; dashboards, hooks, plugins in v2.0 | The local REST API already exists; the CLI is new |

## 10. Features reshaped or deferred, and why

**Seven items from the catalogue change shape so that NWP never claims something it cannot know.**

| Vision item | Change | Reason |
| --- | --- | --- |
| Foreground vs Background Usage | Replace with "active, idle, locked" user state from logind; add window focus only on X11, labelled | Wayland does not let other programs read the focused window |
| Application Session Tracking | Define a session as the lifetime of the app's cgroup | Window or login sessions are not observable from the collector |
| Application Traffic Classification | Show "estimated" with the rule and confidence used | Encrypted traffic gives only rate, direction and duration |
| Application Activity Score | Keep, but only as a weighted sum whose parts are always shown; no "risk" wording | Matches your own "explainable, not unexplained numbers" requirement |
| Domain Tracking | Names come from observed DNS only; DoH and DoT show as "unknown" | Encrypted DNS hides names; reverse lookup is optional and off by default for privacy |
| Floating widget, always-on-top, click-through, overlay | Full support on X11 and where the compositor offers layer-shell (KDE and wlroots-based); GNOME gets an indicator or extension instead | Mutter does not offer layer-shell [HYPOTHESIS: re-check the status when v1.5 starts] |
| Destination Location Lookup | Optional, offline database bundled or user-supplied, off by default | Online lookups would break the offline promise |

**Deferred past v1.0 and held for your approval**

- **Application Network Blocking, scheduled restrictions, pause mode, interface restrictions (catalogue 12).** The risk is a bug that cuts someone's network or a privileged rule engine that others can abuse. Build as a separate optional component (for example `netwatch-guard`) with its own polkit action, using nftables cgroup matching [HYPOTHESIS: verify the exact match support on the kernels you target]. Do not link it into `netwatchd`.
- **Plugin System and Custom Scripts (23).** A plugin running inside the root daemon is an escalation path. Start with integration hooks that run as the user, fed by the local API, and decide on plugins only after v1.5 with a sandbox design.
- **Custom Dashboards and Advanced Notification Rules (23).** Valuable but they need a settled widget and metric API; wait for v2.0.

**Merged, not dropped.** Duplicate entries (launch correlation, idle, comparison, fingerprints, budgets) are built once and linked from each section, as section 3 explained.

## 11. Risks, governance and decisions for you

**The biggest threat to v1.0 is not a technical limit but adding catalogue items before Stage 0 and v0.10 are finished.** The plan guards against that with the gates in section 5.

| Risk | Likelihood | Effect | Mitigation |
| --- | --- | --- | --- |
| Scope creep pulls v1.0 past April | High | Release slips months | Cut rule; "v1.0 contains no new data source" |
| Upload accounting cannot be corrected | Medium | Upload numbers look wrong next to router counters | Spike in Stage 0; if it fails, show the gap openly and keep a calibrated note |
| Distro differences (kernel, cgroup, polkit, packaging) | Medium | Works on CachyOS only | VM matrix in v0.9.3 and again at rc.1 |
| IPv6 per-connection tracking stays blocked | Medium | Connection table misses IPv6 traffic | Label IPv4-only; keep totals correct |
| Agent-written docs claim more than code does (as in G1) | Medium | Loss of trust | Rule: CHANGELOG and decisions list only tested behavior |
| Privileged code grows (probes, blocking, plugins) | Low now, high later | Security incidents | Separate components, security-review entry before each |

**Decisions I need from you** (record each in `docs/decisions.md`; the agent should not start the related work before)

- [ ] Approve Stage 0 and the beta.5 scope (section 2).
- [ ] Approve "v1.0 has no new data source" and the v0.10 and v0.11 scope (section 7).
- [ ] Approve the order of v1.1 to v1.5 (section 8), or reorder.
- [ ] Approve the reshaped items in section 10 (foreground vs background, sessions, classification, domains).
- [ ] Decide whether app blocking and plugins are in the product vision at all, or stay out permanently.
- [ ] Keep NWP fully free through v1.0 (current decision), and revisit tiers when your other apps reach beta.

**Next documents to produce** once you approve: (1) an agent prompt that merges this plan into `docs/ROADMAP.md`, `CHECKLIST.md` and `docs/decisions.md`, with per-release acceptance tests; (2) one design note per new data source before its release starts (DNS observation, logind idle state, probes).
