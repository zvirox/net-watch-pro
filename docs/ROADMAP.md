# Net Watch Pro Roadmap to v1.0 and Beyond

**Version:** 1.0 · **Date:** 2026-11-23  
**Status:** Approved for documentation merge (implementation governed by `AGENTS.md`)

---

## 1. Purpose and Guiding Principles

The vision for Net Watch Pro is a comprehensive, offline, explainable network ledger for Linux. Most Linux tools show either live totals (bandwidth meters) or low-level packet captures (Wireshark-style); few provide per-app history, budgets, and evidence-backed explanations together without relying on cloud services.

The vision document spans 23 catalogue sections containing over 300 line items. To maintain software stability, security boundaries, and high test coverage, implementation is structured strictly by **data dependency** (what the collector can truthfully know) rather than catalogue order.

### Core Architecture Rule: "v1.0 Contains No New Data Source"

Through stable release v1.0.0, Net Watch Pro will rely exclusively on the data sources already present in the architecture:
- Interface counters via sysfs (`/sys/class/net/*`).
- Per-cgroup and per-socket byte accounting via the kernel Aya eBPF collector (`cgroup_skb`).
- Process and desktop metadata via local procfs and `.desktop` desktop entry parsing.
- Offline, peer-credential-gated Unix domain socket API.

Features requiring new privileged collectors or operating system hooks (such as DNS packet observation, active ICMP/UDP probes, process window focus tracking, or firewall packet filtering) are staged for post-1.0 releases.

All release dates and delivery windows in this roadmap are planning **targets, not commitments**. Every release is gated strictly by testable acceptance criteria on real hardware.

---

## 2. The Five Data Layers (L0 to L4)

Features can only be built once the underlying data layer is measured and verified. Staging capabilities across these five layers prevents rework and ensures that claims match reality:

```
+-----------------------------------------------------------------------+
| L4: External Observation & Active Probes (v1.2, v1.3)                 |
|     DNS packet observation, active latency/loss probes, nl80211 stats |
+-----------------------------------------------------------------------+
| L3: Statistical & Baseline Intelligence (v1.1)                        |
|     Shared time-series baseline engine, anomaly detection, scoring    |
+-----------------------------------------------------------------------+
| L2: System Session & User State (v1.1)                                |
|     systemd-logind D-Bus IdleHint/LockedHint, cgroup lifetime         |
+-----------------------------------------------------------------------+
| L1: Per-Cgroup & Per-Socket Accounting (Current - Aya eBPF)           |
|     cgroup_skb ingress/egress, FLOWS map, local rollups store         |
+-----------------------------------------------------------------------+
| L0: Host & Interface Counters (Current - sysfs / procfs)              |
|     sysfs interface rates, system status, peer-credential socket      |
+-----------------------------------------------------------------------+
```

- **Layer 0 (Host Counters):** Live PC throughput, interface breakdown, basic network status. Available today.
- **Layer 1 (Socket & Cgroup Accounting):** Per-app live rates, IPv4 live flow table, 15-minute persistent rollups, attribution remainder. Available today (IPv4 flows; IPv6 per-app bytes; upload GSO spike in progress).
- **Layer 2 (User & Session State):** User active, idle, and screen-locked states via `systemd-logind` D-Bus; application sessions defined by cgroup lifetimes. Staged for v1.1.
- **Layer 3 (Baseline Intelligence):** Local statistical baseline engine computed over persistent SQLite rollups, historical anomaly detection, activity score metrics, traffic fingerprinting. Staged for v1.1.
- **Layer 4 (External Observation & Probes):** Passive DNS response tracking for domain names; active latency, jitter, and packet loss measurement via unprivileged sockets; Wi-Fi link quality via nl80211. Staged for v1.2 and v1.3.

---

## 3. Priority Rubric and Release Gates

Every proposed feature is evaluated against four ordered questions. If a feature fails Question 1, it cannot be scheduled regardless of its perceived value:

1. **Is the data true?** Can the collector or a system source supply it with known accuracy today? If not, it waits for the release that adds that source.
2. **Does it serve the core promise?** Per-app, offline, explainable network history. Features that merely decorate wait.
3. **Value to a typical user:** Casual users checking data consumption first, followed by power users, then security-focused users.
4. **Effort and risk:** New privileged code, kernel hooks, and complex desktop compositor hooks carry the highest cost.

### Priority Classification

| Priority | Meaning | Example Features | Target Release |
|---|---|---|---|
| **P0** | Essential fixes to current plan | Stage 0 gaps G1–G8 (IPv6 accuracy, VM matrix, upload spike) | v0.9.2-beta.5, beta.6 |
| **P1** | Trustworthy v1.0 daily monitor | Budgets, forecasting, alerts, connection explorer, heatmaps, CLI | v0.10, v0.11 |
| **P2** | Standout intelligence (each adds 1 data source) | Anomaly detection, idle investigator, domain intelligence, health | v1.1 to v1.4 |
| **P3** | Platform reach and UI polish | Desktop adapters, panel widgets, custom dashboards | v1.5, v2.0 |
| **P4** | Architectural trust model changes | Application blocking, root plugin execution | v2.0+ (requires owner approval) |

### Mandatory Release Gates

No git tag or package release may occur until every gate is ticked with evidence in `CHECKLIST.md`:

- **Real Hardware Acceptance:** Acceptance tests pass on physical Linux hardware with eBPF attached, not solely on synthetic or seeded fixtures.
- **Truthful Measurement:** Every user-visible number in the UI states how it was derived; every known limitation is documented in both UI tooltips and docs.
- **Automated Verification:** `cargo test`, workspace UI tests (`npm test`), lints, and security checks pass without warnings or regressions.
- **Security Review:** Any addition of privileged execution or capability grants requires an updated entry in `docs/security-review.md` before implementation.
- **Honest Changelog:** `CHANGELOG.md` lists only what the code actually does; any unverified assertion is tagged `[HYPOTHESIS]`.
- **Governance:** Scope additions or contract modifications are recorded in `docs/decisions.md` and approved by the project owner per `AGENTS.md` §3.
- **Cut Rule:** If a release milestone slips by more than two weeks, lowest-priority scope items are dropped rather than delaying the target window.

---

## 4. Release Timeline Overview

All dates listed below are planning **targets, not commitments**.

| Milestone | Target Window | Focus & Primary Deliverable |
|---|---|---|
| **Stage 0 (beta.5)** | Target Dec 6, 2026 | Truth in docs, Stage 0 gaps G1, G2, G5, G7, G8, attribution harness |
| **Stage 0 (beta.6)** | Target Dec 13, 2026 | G4 upload GSO spike outcome, G6 multi-distro VM matrix |
| **v0.9.3 "Trust"** | Target Dec 20, 2026 | Monitoring health, self-diagnostics, data attribution transparency |
| **v0.10 "Plan"** | Target Jan 24, 2027 | Budgets, forecasting, desktop alerts, heatmaps, leaderboard, CLI |
| **v0.11 "Explore"** | Target Mar 7, 2027 | Connection explorer, app profiles, comparison studio, smart archive |
| **v1.0.0-rc.1** | Target Apr 4, 2027 | Feature freeze, distro packaging (.deb, .rpm, PKGBUILD), soak tests |
| **v1.0.0 Stable** | Target Apr 25, 2027 | Stable general availability, signed repositories, full user documentation |
| **v1.1 "Insight"** | Target Jun 2027 | Local baselines, anomaly alerts, idle activity investigator, scores |
| **v1.2 "Destinations"** | Target Aug 2027 | Passive DNS observation, domain tracking, offline destination mapping |
| **v1.3 "Network Health"** | Target Oct 2027 | Active latency/jitter/loss probes, link stats, full network profiles |
| **v1.4 "Time Machine"** | Target Dec 2027 | Unified event store, timeline playback, PDF/scheduled reporting |
| **v1.5 "Desktop"** | Target Q1 2028 | Desktop adapters (Wayland layer-shell, KDE/GNOME panels) |
| **v2.0 "Control & Extend"** | Target Q2 2028 or later | Standalone `netwatch-guard` blocking, user-level integration hooks |

---

## 5. Pre-1.0 Releases: Detailed Scope and Acceptance

### 5.1 Stage 0: v0.9.2-beta.5 (Target Dec 6, 2026)

**Goal:** Reconcile documentation with measured code reality and close public beta audit gaps.

**Scope:**
- **G1:** Correct IPv6 documentation across `CHANGELOG.md`, `docs/decisions.md` (D8), and `README.md` to state that IPv6 traffic is aggregated per app while per-connection IPv6 flows are not tracked.
- **G2:** Reconcile `CHECKLIST.md` with `docs/spike-ebpf-report.md`, recording verified items and unticking unverified GUI items.
- **G3:** Implement `tests/host/attribution.py` running on physical Linux with root and eBPF active to compare per-app accounting with NIC counters.
- **G5:** Run acceptance tests #4 (torrent/UDP), #6 (daemon restart recovery), and #9 (zero outbound traffic with live collector active) on CachyOS host.
- **G7:** Harden polkit helper (`/usr/libexec/netwatch-pro/enable-monitoring`) to exit non-zero with explicit error logs if `usermod` or systemd drop-in generation fails.
- **G8:** Record decision D14 establishing license expiration rechecks as a mandatory precondition before any paid license ships.

**Acceptance Criteria:**
- `grep -i "ipv6" docs/*.md` produces zero claims of per-connection IPv6 flow tracking.
- Running `tests/host/attribution.py` under live root execution prints attributed vs. unattributed percentages alongside daemon CPU and memory consumption.
- Polkit helper execution with a simulated read-only `/etc/systemd` produces an immediate non-zero exit code and diagnostic stderr message.

---

### 5.2 v0.9.3 "Trust" (Target Dec 20, 2026)

**Goal:** Give users total visibility into how Net Watch Pro measures their system and why local counters may differ from routers or ISPs.

**Scope:**
- **G4 & G6 resolution:** Incorporate GSO segment spike outcome for upload accounting and publish the completed VM matrix across Arch, Ubuntu LTS, Fedora, and Debian.
- **Monitoring Health and Self-Diagnostics (Catalogue 23):** Expand the Diagnostics modal into a dedicated screen reporting collector status, attach mode, capability drops, kernel BTF status, and actionable remediation steps.
- **Data Attribution Transparency (Catalogue 23):** In-app documentation view explaining how packets are measured at socket boundaries, loopback exclusions, GSO effects, and router vs. host discrepancies.
- **Data Integrity Validation (Catalogue 22):** Implement `nwp check` CLI diagnostic command verifying SQLite rollups consistency against raw counters.

**Acceptance Criteria:**
- Intentionally disabling eBPF (`NETWATCH_DISABLE_EBPF=1`) causes the UI to clearly state that per-app rates are unavailable while PC throughput continues functioning, without crashing or throwing unhandled errors.
- Running `nwp check` identifies intentionally corrupted rollup rows and offers safe database repair.
- The daemon installs and starts cleanly on clean installations of Ubuntu 24.04 LTS, Fedora 41, Debian 12, and Arch Linux.

---

### 5.3 v0.10 "Plan" (Target Jan 24, 2027)

**Goal:** Turn historical rollups into actionable usage limits, budget projections, and administrative automation.

**Scope:**
- **Smart Budget System (Catalogue 7, 8):** Multi-tier quotas (total, download, upload), per-interface budgets, per-app limits, custom cycle resets, configurable warning (e.g. 80%) and critical (e.g. 95%) thresholds.
- **Forecasting (Catalogue 7):** Linear projection with weekday weighting predicting month-end usage, date of budget exhaustion, and fair-use policy (FUP) overage dates. Calculation math is fully exposed to the user.
- **Desktop Notifications (Catalogue 11, 23):** System notifications for threshold crossings, quiet-hours filtering, and rate-limited alert dispatch.
- **Usage Leaderboards & Heatmaps (Catalogue 17, 18):** Top applications ranked by volume and peak throughput; GitHub-style daily calendar heatmap and hourly breakdown generated directly from rollups.
- **CLI and Headless Export (Catalogue 13, 23):** Stable `nwp` command-line interface supporting `nwp status`, `nwp top`, and scriptable JSON/CSV data extraction.
- **Compact Tray Monitor (Catalogue 11):** StatusNotifierItem tray menu showing live bandwidth and compact floating window mode.

**Acceptance Criteria:**
- Setting a 10 GB cycle budget triggers a desktop notification when seeded usage reaches exactly 8.0 GB (80%).
- The forecasting engine displays the mathematical formula and historical input points used to calculate the predicted exhaustion date.
- The `nwp top --json` command outputs valid JSON adhering to the public schema within 200 ms on a database with 90 days of history.

---

### 5.4 v0.11 "Explore" (Target Mar 7, 2027)

**Goal:** Deep forensic inspection of active and past network activity without modifying network traffic.

**Scope:**
- **Network Forensic Explorer (Catalogue 4):** Filterable, searchable historical connection inventory (protocol, IP, port, duration, data volume, process name). IPv4 supported; IPv6 included only if Stage 0 spike passed.
- **Application Network Profile Page (Catalogue 1, 2):** Dedicated view per application displaying lifetime volume, daily averages, peak rates, active cgroup duration, and upload/download ratios. Sessions are explicitly labelled as cgroup lifetimes.
- **Traffic Comparison Studio (Catalogue 16):** Side-by-side comparison of applications, time periods (e.g. this week vs. last week), interfaces, and upload vs. download distributions.
- **Network Identity Basics (Catalogue 10):** Tracking interface state changes, default gateway switches, and manual user trust categorization (e.g. marking an interface as "metered").
- **Smart History Archive & Portability (Catalogue 13, 22):** Tiered historical retention (15-minute rollups for 90 days, daily rollups for 1 year), archive size management, and full database export/import with pre-restore checksum validation.

**Acceptance Criteria:**
- Performing a complex multi-column filter query across 100,000 historical connection records returns in less than 500 ms on reference hardware.
- Exporting a backup from one host and restoring it on a clean host reproduces identical per-app rollup totals to the byte.
- App detail screens explicitly display "Session duration represents cgroup lifetime" in compliance with truthful reporting rules.

---

### 5.5 v1.0.0-rc.1 and v1.0.0 Stable (Target Apr 4 to Apr 25, 2027)

**Goal:** Rigorous stabilization, packaging verification, performance profiling, and production release.

**Scope:**
- **Strict Feature Freeze:** Zero new feature additions after v1.0.0-rc.1.
- **Native Packaging:** Tested distribution packages:
  - Arch Linux: AUR `net-watch-pro` and `net-watch-pro-bin` with automated systemd unit installation.
  - Debian/Ubuntu: `.deb` package with signed repository and proper maintainer scripts.
  - Fedora: `.rpm` package signed and tested against current Fedora versions.
- **Endurance Testing:** 14-day continuous soak test on physical hardware with eBPF collector running, monitoring for memory leaks or sqlite lock contention.
- **Performance Budget Compliance:** Verified daemon overhead under heavy network saturation: CPU usage <= 3.0% single core, RSS memory <= 150 MiB.
- **Documentation & Security Audit:** Updated `docs/security-review.md`, finalized User Guide, comprehensive offline troubleshooting manual.

**Acceptance Criteria:**
- 14-day soak test completes with zero daemon crashes, zero unhandled worker panics, and flat RSS memory usage.
- All acceptance gates in `CHECKLIST.md` are signed with real hardware execution evidence.
- Clean package install, service start, onboarding flow, and clean package purge verified on Ubuntu 24.04, Fedora 41, and Arch Linux.

---

## 6. Post-1.0 Releases: One Data Source Per Release

Releases after v1.0 expand the product's intelligence capabilities. To maintain safety and prevent architectural drift, each major post-1.0 milestone introduces **exactly one new data source**:

### v1.1 "Insight" (Target Jun 2027)
- **New Data Source:** Local statistical baseline computation over existing SQLite rollups, combined with `systemd-logind` D-Bus user activity state (`IdleHint`, `LockedHint`).
- **Delivered Capabilities:**
  - Automated anomaly alerts (unusual upload spikes, unexpected off-hours activity, new background processes).
  - Idle Activity Investigator (Catalogue 19): tracking which applications consume data while the screen is locked or the seat is idle.
  - Explainable Application Activity Score (Catalogue 2): transparent weighted sum (0–100) reflecting relative data volume, connection churn, and off-hours activity (no arbitrary "risk" or "malware" labels).
  - Application Traffic Fingerprinting (Catalogue 21): empirical transfer profiles (continuous stream vs. periodic burst).

### v1.2 "Destinations" (Target Aug 2027)
- **New Data Source:** Passive local DNS packet observation (capturing local UDP/TCP port 53 responses or local systemd-resolved monitor socket).
- **Delivered Capabilities:**
  - Domain intelligence (Catalogue 6): mapping remote IP addresses to human-readable domain names.
  - Transparent limits: DNS-over-HTTPS (DoH) and DNS-over-TLS (DoT) destinations are labelled "unknown" rather than guessed.
  - Destination rankings and domain leaderboards (Catalogue 18).
  - Optional, strictly offline IP geolocation database (bundled or user-supplied, disabled by default to protect privacy).

### v1.3 "Network Health" (Target Oct 2027)
- **New Data Source:** Active network probe engine (unprivileged ICMP echo / UDP sockets) and wireless link metrics (`/proc/net/wireless` and nl80211).
- **Delivered Capabilities:**
  - Network Performance Intelligence (Catalogue 9): continuous latency, jitter, packet loss, and gateway health monitoring.
  - Contextual Network Profiles (Catalogue 10): automatic detection of Wi-Fi SSIDs, customized per-network budgets (e.g. mobile hotspot vs. home fiber), and connection quality scoring.

### v1.4 "Time Machine" (Target Dec 2027)
- **New Data Source:** Unified local event store synchronizing network anomalies, connection bursts, and system state transitions.
- **Delivered Capabilities:**
  - Interactive Network Time Machine (Catalogue 5, 15): timeline scrubbing allowing users to inspect the exact network state at any historical moment.
  - Multi-entity correlation (Catalogue 20) linking app launches to sudden traffic surges.
  - Scheduled PDF reports and chart image exports (Catalogue 13).

### v1.5 "Desktop Adapters" (Target Q1 2028)
- **New Data Source:** Compositor and desktop environment adapter layers (Wayland layer-shell protocols and desktop-specific D-Bus interfaces).
- **Delivered Capabilities:**
  - Desktop-integrated floating widgets, GNOME Shell extensions, and KDE Plasma applets.
  - Global configurable hotkeys and quick-view panels.

### v2.0 "Control and Extend" (Target Q2 2028 or later)
- **New Data Source:** Standalone privileged traffic control daemon (`netwatch-guard`) using nftables cgroup matching; subject to explicit owner approval.
- **Delivered Capabilities:**
  - Application network blocking, metered data pausing, and scheduled restrictions (Catalogue 12).
  - User-space integration hooks and isolated extension sandbox (Catalogue 23).
  - *Architectural invariant:* `netwatch-guard` remains completely separated from `netwatchd` so monitoring bugs cannot disrupt network traffic.

---

## 7. Vision Map: 23 Catalogue Sections

Every section from the product vision document is accounted for in this roadmap. Several sections are split across releases because their sub-features depend on different data layers:

| # | Vision Section Title | Delivered In | Scope & Architectural Notes |
|---|---|---|---|
| 1 | Deep Application Intelligence | v0.11, v1.1 | v0.11: profile page, cgroup sessions; v1.1: idle tracking, fingerprinting. |
| 2 | Application Behavior Score | v0.11, v1.1 | v0.11: basic metrics; v1.1: baseline-relative explainable score (no risk labels). |
| 3 | Anomaly Detection and Alerts | v1.1 | Threshold alerts ship in v0.10; statistical baseline anomalies ship in v1.1. |
| 4 | Network Forensic Explorer | v0.11, v1.2 | v0.11: IPv4 connections; v1.2: domain name enrichment via DNS tracking. |
| 5 | Connection Timeline & Investigation | v1.2, v1.4 | v1.2: DNS correlation; v1.4: unified scrubbable timeline. |
| 6 | Domain and Destination Intelligence | v1.2 | Passive DNS observation; encrypted DNS labelled unknown; offline geo. |
| 7 | Forecasting and Usage Planning | v0.10 | Cycle forecasting and FUP calculation; app-level forecasting in v1.1. |
| 8 | Smart Budget System | v0.10 | Quotas, thresholds, daily allowance, multi-interface budgets. |
| 9 | Network Performance Intelligence | v1.3 | Latency, jitter, loss, gateway health, Wi-Fi link speed, error stats. |
| 10 | Network Profiles and Context | v0.11, v1.3 | v0.11: network identity, metered flags; v1.3: full profiles and policies. |
| 11 | Interface and System Integration | v0.10, v1.5 | v0.10: tray icon, compact mode; v1.5: Wayland/KDE/GNOME desktop widgets. |
| 12 | Advanced Traffic Controls | v2.0 (only with approval) | Isolated `netwatch-guard` firewall component; excluded from v1.0. |
| 13 | Reporting, Automation, Export | v0.10, v0.11, v1.4 | v0.10: CLI/JSON/CSV; v0.11: backup/restore; v1.4: PDF/scheduled reports. |
| 14 | Network Intelligence Engine | v1.1 | Shared local baseline engine serving sections 2, 3, 19, and 21. |
| 15 | Network Time Machine | v1.4 | Historical playback, state comparison, scrubbable investigation view. |
| 16 | Traffic Comparison Studio | v0.11, v1.3, v1.4 | Side-by-side comparison across apps, intervals, interfaces, and profiles. |
| 17 | Usage Heatmap | v0.10 | Daily calendar and hourly distribution matrices derived from rollups. |
| 18 | Usage Leaderboard | v0.10, v1.2 | v0.10: top apps; v1.2: top destinations and domains. |
| 19 | Idle Activity Investigator | v1.1 | Off-hours and screen-locked data tracking via logind D-Bus. |
| 20 | Event Correlation Engine | v1.4 | Correlating application lifecycle events with traffic spikes. |
| 21 | Application Traffic Fingerprinting | v1.1 | Transfer profiles (rate, duration, ratio) presented as factual metrics. |
| 22 | Smart History Archive | v0.9.3, v0.11 | v0.9.3: integrity checks; v0.11: tiered retention and downsampling. |
| 23 | Additional Ultimate-Level Features | v0.9.3–v0.10, v0.11, v1.4, v2.0 | Diagnostics (v0.9.3), CLI (v0.10), backup (v0.11), tags (v1.4), plugins (v2.0). |

*See [docs/FEATURE-CATALOGUE.md](FEATURE-CATALOGUE.md) for the complete index and section details.*

---

## 8. Reshaped and Deferred Features

To maintain strict truthfulness in reporting and honor technical sandbox constraints on Linux, seven catalogue items have been reshaped, and several items deferred past v1.0:

### Reshaped Items

| Feature | Reshaped Implementation | Reason |
|---|---|---|
| **Foreground vs. Background Usage** | Replaced with "active", "idle", and "locked" system states derived from `systemd-logind` D-Bus signals. Window focus is supported on X11 only as an optional, labelled feature. | Modern Wayland compositors (Mutter, KWin, wlroots) intentionally prevent unprivileged applications from monitoring focused windows for security and privacy. |
| **Application Session Tracking** | Defined strictly as the lifetime of the application's kernel cgroup (creation to exit). | Window-level and desktop-level sessions cannot be reliably observed from passive network socket collectors. |
| **Traffic Classification** | Modelled as heuristic classification labelled "estimated", with the rule and confidence score visibly displayed. | Payload inspection is impossible on modern encrypted traffic (TLS 1.3, QUIC). Passive monitors can only observe rate, packet size, and duration. |
| **Activity Scoring** | Calculated as an explainable, weighted arithmetic sum of measured factors; arbitrary "threat", "risk", or "security" scores are forbidden. | Conforms to the core design principle: evidence-based numbers that explain themselves rather than opaque proprietary scores. |
| **Domain Intelligence** | Domain names are harvested exclusively from observed local DNS responses. Encrypted DNS (DoH/DoT) queries are displayed as "unknown". | Reverse PTR lookups would generate outbound network traffic (violating the zero-outbound contract) and often yield incorrect CDN hostnames. |
| **Desktop Overlays & Widgets** | Native layer-shell widgets on supported compositors; standard panel indicators on GNOME Shell. | Wayland compositors do not permit global floating windows or click-through overlays without compositor-specific privileged protocols. |
| **Destination Location Lookup** | Implemented strictly via offline local MaxMind/IP2Location databases, disabled by default. | Online geolocation APIs would violate the zero-outbound architectural guarantee. |

### Deferred Past v1.0

- **Traffic Control & Firewalling (Catalogue 12):** Network blocking, bandwidth throttling, and pause controls are deferred to v2.0. If approved by the project owner, this capability will be implemented as an independent privileged daemon (`netwatch-guard`) interacting with nftables, keeping `netwatchd` strictly read-only.
- **Root Plugins & Custom Scripts (Catalogue 23):** Arbitrary script execution inside the root daemon creates a severe privilege escalation vector. Extension mechanisms will be restricted to unprivileged user-space webhooks fed by the local REST/WebSocket API.
- **Custom Visual Dashboards (Catalogue 23):** Deferred to v2.0 to allow UI component APIs to stabilize.
