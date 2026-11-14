# Net Watch Pro — Product Requirements Document

**Version:** 1.0 (MVP scope)  
**Date:** 2026-11-14  
**Status:** Draft for build  
**Owner:** Product  
**Related docs:** `REAPM.md` (competitive/technical teardown of the reference application firewall)

---

## 1. Vision

Net Watch Pro is a network traffic monitor and data-usage tracker for Linux. It shows every application's network activity in real time, accounts data usage per app and per period, and keeps a searchable local history — all on-device, with no cloud and no telemetry.

**One-line pitch:** "See exactly which apps use your data and your network — privately, locally, in real time."

**Product strategy:** Monitor-first. V1.0 ships as a stable, polished, monitor-only product. Full application-firewall capability (blocking, rules, prompts, DNS control) is added step by step in later major versions, reusing the same interception foundation.

---

## 2. Problem Statement

Linux users today have no simple, trustworthy answer to basic questions:

- **Which app is using my data right now?** Existing tools are either terminal-only (`iftop`, `nethogs`, `vnstat`), per-interface rather than per-application, or require assembling several tools.
- **How much data did this app use this month?** Data caps, metered connections (mobile hotspots, satellite, capped ISP plans) are common, but Linux lacks a per-app usage tracker with history.
- **What is this app connecting to?** Discovering an app's remote endpoints requires `ss`/`netstat` spelunking and correlating with `/proc` by hand.
- **Can I trust the tool itself?** Privacy-conscious users are wary of network tools that phone home. There is demand for a strictly local, auditable, open monitoring app.

The reference app analyzed in REAPM.md proves demand for a polished desktop UI over per-app network data, but it is firewall-first (root interception with verdict engine), heavyweight, and Go/Angular-based. No leading product occupies the "lightweight, monitor-only, modern-stack, privacy-first" niche on Linux.

---

## 3. Target Users

### Persona 1 — "Privacy-conscious Power User" (primary; MVP optimizes for this persona)

- **Profile:** Runs Arch/CachyOS or an Ubuntu/Fedora-family distro as a daily driver. Cares about telemetry, data caps, and knowing what apps do. Comfortable installing packages and granting admin once, but does not want to babysit terminal tools.
- **Jobs to be done:** Identify data-hungry apps; spot unknown/suspicious connections; verify that an app isn't phoning home.
- **Success for them:** Open the dashboard, see top talkers and live connections within seconds; trust the tool because everything is local.

### Persona 2 — "Metered-Connection User"

- **Profile:** Mobile hotspot, LTE/5G dongle, capped ISP plan, or rural connection. Needs to avoid overage charges.
- **Jobs to be done:** Know today's/month's total usage; see which app is draining the cap; get a heads-up before exceeding a budget.
- **Success for them:** A glanceable "used so far" number per app and total, with history to explain last month's bill.

### Persona 3 — "Developer / Sysadmin" (secondary; drives V2 depth)

- **Profile:** Wants per-connection detail: process, PID, remote IP:port, protocol, DNS names where available, and raw data export for further analysis.
- **Jobs to be done:** Correlate network activity with processes; export usage data to spreadsheets/scripts.
- **Success for them:** Searchable 90-day history with filters, CSV/JSON export, and stable local data they can script against.

---

## 4. Goals and Non-Goals

### Goals (MVP)

1. **Accurate per-app accounting:** bytes up/down attributed to the correct application, verified against system counters within ±5%.
2. **Live visibility:** a real-time connections view (app, PID, protocol, remote endpoint, rate) with sub-second refresh.
3. **Local-only data:** everything in a local SQLite database; no network calls by the app itself; default 90-day retention (configurable), automatic pruning.
4. **Effortless install:** one-command install on Arch/CachyOS (AUR), deb-based (apt repo), and rpm-based (dnf repo) distros; daemon runs as a systemd service; UI is unprivileged.
5. **Low overhead:** < 3% CPU and < 150 MB RSS for the daemon under typical desktop load; negligible packet path cost (eBPF observe, no blocking).
6. **Modern, fast UI:** Svelte 5 + shadcn-svelte design system, ECharts visualizations, dark/light themes.
7. **Freemium foundation:** free tier = full live monitoring + per-app usage + basic history; Pro tier (license key) = advanced history search + data export. License validation ships in MVP.

### Non-Goals (MVP)

- **No blocking / firewall / prompts.** Monitor-only until stable V1.0. Firewall features are explicitly deferred and phased in after V1.0 (see §12 Roadmap).
- **No DNS interception or resolver changes.** Do not reroute or rewrite DNS in MVP (reverse-DNS display only).
- **No Windows/macOS support.** Linux only.
- **No cloud sync, accounts, or team dashboards.**
- **No packet capture/pcap export** (potential Later feature).
- **No container/VM internal visibility** beyond attributing traffic to the host process/runtime.

---

## 5. Tech Stack (mandated)

| Layer | Choice | Notes |
|---|---|---|
| Desktop shell | **Tauri 2** | Unprivileged UI process; tray icon; window state; single-instance |
| Frontend | **Svelte 5** | Fast, small bundle; runes-based state |
| Components | **shadcn-svelte** | Design system, theming (dark/light), accessible primitives |
| Charts | **ECharts** | Usage timelines, top-talker bars, sparklines |
| Backend | **Rust + Tokio** | Daemon core; async pipeline; HTTP/WS local API for the UI |
| Monitoring | **Aya eBPF** | Kernel-side traffic observation (cgroup/socket hooks), no packet rewriting |
| Database | **SQLite** | Local history/usage store; WAL mode; automatic pruning |
| Platforms | **Arch / CachyOS; deb & rpm distros** | AUR PKGBUILD + native package + apt repo + dnf repo |

**Architecture shape (no code, contract only):**
- `netwatchd` (root systemd service, Rust): loads Aya eBPF programs, attributes traffic to processes via eBPF metadata + `/proc` enrichment, writes usage records to SQLite, serves a localhost-only authenticated WS/HTTP API to the UI, exposes a healthy-status endpoint. Hardened unit (like the reference app: `ProtectSystem`, `PrivateTmp`, `RestrictAddressFamilies`, capability bounding to `CAP_BPF`/`CAP_PERFMON`/`CAP_NET_ADMIN` where supported).
- UI (Tauri 2, user context): connects to the daemon's local WS API; graceful degraded states when daemon is stopped or lacks privileges.
- IPC contract: versioned JSON over localhost WS with token auth; UI never requires root.

**Kernel requirements (documented, checked at install):** CO-RE BTF available (typical on Arch/CachyOS current kernels, Ubuntu 22.04+, Fedora 36+); installer verifies and warns with actionable guidance otherwise.

---

## 6. Feature Set

### 6.1 MVP — Must-have (ships V1.0)

1. **Live Connections view** — real-time table of active connections: app name/icon, PID, protocol (TCP/UDP), local/remote IP:port, current rate, cumulative bytes; pause/freeze snapshot; sort and search within view.
2. **Per-app usage accounting** — data used per application for today / this week / this month, with up/down split; "Top apps" ranked list and total system usage.
3. **Dashboard** — ECharts timeline of usage (hour/day granularity toggle), top-talker bar chart, total-usage headline, per-interface breakdown (treat metered interfaces distinctly, e.g. hotspot detection is informational).
4. **App detail page** — per-app drill-down: usage over time, open connections, endpoint summary (remote IPs/hosts seen), first/last seen.
5. **Local history** — SQLite-backed connection log with the fields above; default 90-day retention, user-configurable, automatic pruning; basic search by app name.
6. **Daemon + install experience** — systemd service install/enable via pkexec (once), health check, clear degraded states ("Daemon stopped", "eBPF unavailable", "Permissions missing") with self-help text.
7. **Tray integration** — tray icon with today's total usage tooltip; menu: open dashboard, pause UI notifications, quit UI (does not stop daemon — mirrors the reference app's safe pattern).
8. **Settings (lean)** — retention length, theme, start-minimized, tray behavior, metered-interface labeling, daemon start on boot.
9. **Freemium mechanics** — free tier as above; Pro license key unlocks: advanced history search (multi-filter: app, date range, remote host/IP, port, protocol), CSV/JSON export, retention beyond 90 days (up to 1 year). Pro features clearly gated in-UI with upgrade prompt.
10. **Updates & packaging** — AUR package, .deb + apt repo, .rpm + dnf repo; signed updates; uninstall cleanly removes daemon + data (with opt-in data retention).

### 6.2 V2 — Should-have (next release train)

1. **Usage alerts/budgets** — per-app and total daily/monthly budgets with desktop notifications; metered-interface aware.
2. **Advanced history search (free-tier visibility, Pro depth)** — saved filters/queries.
3. **DNS name resolution display** — resolve remote IPs to PTR/hostnames shown alongside IPs (no interception).
4. **Scheduled reports** — weekly usage summary as notification or file.
5. **Interface niceties** — per-interface filters, speedtest-aware "current interface" widget, import of process icons for unknown apps.
6. **Process-tree attribution improvements** — better handling of Flatpak/Snap/AppImage, script interpreters, systemd services.
7. **Pause/resume monitoring** — temporarily suspend recording (privacy mode) with tray toggle.

### 6.3 Later — Nice-to-have

1. **Firewall phase-in (post-1.0 program, step by step per user decision):**
   - Step 1: read-only "what would be blocked" rule preview engine.
   - Step 2: per-app allow/block rules with user prompts (verdict pipeline reusing the eBPF hooks, upgraded to blocking programs).
   - Step 3: scope rules (localhost/LAN/Internet), endpoint rules (domain/IP/Country).
   - Step 4: secure DNS control (DoT/DoH routing + rebinding protection) — only if product strategy supports it.
2. **Pcap capture export** for selected connections (pro/dev tooling).
3. **Team/fleet reporting** (opt-in, out of scope until validated demand).
4. **Wayland status extension / GNOME/KDE shell integration.**
5. **Optional encrypted sync of settings between a user's own machines.**

---

## 7. User Stories

### MVP (must-have)

- As a privacy-conscious user, I want to see every app's live network connections in one view, so that I immediately know what is talking to the internet.
- As a privacy-conscious user, I want each connection labeled with the owning application, so that I don't have to map PIDs to programs myself.
- As a metered-connection user, I want to see today's and this month's data usage total, so that I can stay under my cap.
- As a metered-connection user, I want per-app usage ranking, so that I can find the app draining my data.
- As a user, I want a dashboard chart of usage over time, so that I can understand patterns at a glance.
- As a user, I want to click any app and see its detail page, so that I can investigate what it connects to and when.
- As a user, I want a 90-day searchable history, so that I can answer "which app used data last week?" without maintaining anything.
- As a user, I want clear status when the daemon isn't running or lacks permissions, so that I can fix it without reading logs.
- As a user, I want to install once with a package manager and never think about permissions again, so that monitoring "just works" after reboot.
- As a privacy-conscious user, I want all data stored locally with automatic pruning, so that my history is mine and doesn't grow forever.
- As a privacy-conscious user, I want a tray icon showing today's usage, so that I have a passive glanceable indicator.
- As a user, I want dark and light themes, so that the app fits my desktop.
- As a Pro user, I want to filter history by app, date, remote host, port and protocol, so that I can answer specific questions quickly.
- As a Pro user, I want to export usage and connection data to CSV/JSON, so that I can analyze or archive it myself.
- As a user, I want a quit action that closes the UI but keeps the daemon running, so that I don't accidentally stop my usage tracking.

### V2 (should-have)

- As a metered-connection user, I want to set a monthly total budget and get a notification near the limit, so that I avoid overage charges.
- As a metered-connection user, I want per-app budgets, so that I can contain a specific data-hungry app.
- As a user, I want remote IPs shown with resolved hostnames, so that connection lists are readable.
- As a user, I want a weekly usage summary, so that I can review without opening the app.
- As a privacy-conscious user, I want a "pause recording" toggle, so that I can temporarily stop history capture.

---

## 8. Key User Flows

### Flow 1 — First-run install & onboarding (target: < 3 minutes to a working dashboard)

1. User installs via AUR (`paru -S net-watch-pro`), or adds the apt/dnf repo and installs the package.
2. Package post-install registers `netwatchd.service` (disabled by default) and the desktop entry.
3. User launches Net Watch Pro from the app menu.
4. UI detects daemon not active → onboarding card explains monitoring requires a system service → user clicks "Enable monitoring" → `pkexec` prompt (one admin authorization) → daemon installed/enabled/started.
5. Health check verifies: service active, eBPF programs attached, SQLite writable, kernel compatible.
6. On success: onboarding shows a 3-card summary (What we collect / Where it's stored (local) / How to remove), then lands on Dashboard with live data. First data appears within seconds.
7. On failure: specific error state with actionable guidance (e.g., "Kernel 5.15 lacks BTF — see docs link"), retry button, and a "Run without monitoring" demo mode using seeded sample data so the UI remains explorable.

### Flow 2 — Daily glance (dashboard)

1. User opens app (or clicks tray icon) → Dashboard.
2. Headline: total usage today + this month; sparkline of last 24 h.
3. Top apps bar chart (up/down); usage timeline with day/hour toggle; interface breakdown.
4. Clicking any bar routes to that app's detail page.

### Flow 3 — Investigate an app

1. From dashboard or Live view, user opens app detail.
2. Sees: usage over time, current connections, recent connection history (scrollable), remote endpoint summary.
3. Uses in-page search to find a specific remote host/port.
4. (Pro) Applies filters, exports the filtered result to CSV.

### Flow 4 — History search & export (Pro)

1. User opens History.
2. Free tier: search by app name; sees last 90 days.
3. Upgrade prompt explains Pro (multi-filter search + export + longer retention) with license entry.
4. User enters license key → validation (offline-capable, signed key) → Pro features unlock instantly, no restart.
5. User applies multi-filter (app + date range + remote host + protocol) → results table → Export → choose CSV/JSON → file saved via native dialog.

### Flow 5 — Daemon stopped / troubleshooting

1. UI shows persistent banner: "Monitoring stopped — daemon not running."
2. Banner actions: "Start daemon" (pkexec), "Open diagnostics" (shows health checks + journal excerpt).
3. User restarts → health re-check → banner clears; UI reconnects via WS automatically.

### Flow 6 — Quitting UI (not the daemon)

1. Tray menu → Quit.
2. Confirm dialog explains: closing the UI does not stop background monitoring; "Also stop monitoring" checkbox (pkexec) for those who want full stop.

---

## 9. Success Metrics

### Activation (first session)
- **Time-to-dashboard:** median < 3 min from package install to first live data.
- **Onboarding completion:** ≥ 80% of installs complete daemon enablement on first run.
- **eBPF attach success rate:** ≥ 95% on supported kernels (tracked via anonymous-free local diagnostics the user can opt to share manually).

### Engagement
- **WAU/MAU ≥ 40%** of active installs (tray + dashboard glance model should drive passive retention).
- **Dashboard sessions/user/week ≥ 5** (short glance sessions are fine).
- **App detail page views/session ≥ 1** for engaged users.

### Quality
- **Daemon crash-free sessions ≥ 99.5%** (7-day window).
- **Accounting accuracy:** per-app daily totals within ±5% of system interface counters in test harness; zero unresolved mis-attribution reports at launch.
- **Overhead:** ≤ 3% CPU / ≤ 150 MB RSS daemon p95 under synthetic 50-connection load.
- **WS reconnect:** UI auto-recovers within 5 s of daemon restart, no stale-state bugs.

### Business (freemium)
- **Pro conversion ≥ 3%** of weekly active installs by month 3.
- **Trial-to-paid retention:** license keys with 60-day+ continued WAU ≥ 70%.
- **Support burden:** < 5 tickets/100 installs; top 3 issues documented in self-help.

### Trust (privacy positioning)
- **Zero outbound connections** by the app itself, verifiable by users with the app's own Live view (dogfooding claim — must be true).
- **Uninstall completeness:** 100% removal of service files on uninstall; data removal opt-in.

---

## 10. Risks, Assumptions, Open Questions

### Risks

| # | Risk | Impact | Mitigation |
|---|---|---|---|
| R1 | Kernel variance across Arch (rolling) vs Ubuntu LTS vs Fedora breaks eBPF attach | High — core function fails | Pin supported kernel floor (5.10+ w/ BTF), CI matrix across the 3 distro families, installer pre-flight checks, graceful degraded mode |
| R2 | Root daemon attack surface (local WS API, file I/O) | High — security/PR | Localhost-only bind, token auth, hardened systemd unit, capability bounding, minimal endpoints; external security review before 1.0 |
| R3 | Mis-attribution edge cases (short-lived processes, containers, Flatpak/Snap, scripts) | Medium — trust | Attribution via eBPF socket metadata + `/proc` enrichment; mark confidence levels in UI; process improvements in V2 |
| R4 | Data growth in SQLite (90-day connection log) | Medium — disk/perf | WAL + aggressive indexing, record compaction (aggregate old connections into rollups), configurable retention |
| R5 | Wayland/X11 tray & window quirks across DEs (GNOME tray, KDE) | Medium — UX | Tauri tray with DE-specific test matrix; document GNOME extension requirement |
| R6 | VPN/overlay interfaces distort per-app attribution or totals (tunnel double-count) | Medium — accuracy | Treat tunnel interfaces explicitly; deduplicate tun traffic; label VPN sessions |
| R7 | Freemium license enforcement on (potentially) open-source code | Medium — business | Decide licensing model first (OQ1); design key validation as offline-signed; accept best-effort enforcement |
| R8 | Users expect firewall because category leader has it | Medium — expectations | Positioning: "monitor-first, firewall later"; roadmap visible in-app |

### Assumptions

1. Target distros ship kernels ≥ 5.10 with BTF (verified for Arch/CachyOS current; Ubuntu 22.04+, Fedora 36+).
2. One-time admin authorization (pkexec) is acceptable UX for install/enable.
3. SQLite is sufficient for 90-day-per-host connection logs on desktop hardware.
4. Aya + Tokio stack can meet the < 3% CPU overhead bar in observe-only mode.
5. Users tolerate per-app attribution at process granularity (not per-tab/per-service within a browser).

### Open Questions

1. **License model:** fully open-source (GPL/AGPL) with paid prebuilt/binary tier, or source-available? This affects both enforcement (R7) and community trust. **Decide before MVP build.**
2. **Paid tier exact boundary:** is export alone a strong enough differentiator, or should advanced search also be free and export+retention+alerts paid? Needs pricing research.
3. **Daemon API stability:** publish the local API for third-party tools (like the reference app's `app/auth` flow) in MVP, or keep private until V2?
4. **Demo mode policy:** ship sample-data mode in MVP (nice onboarding) or cut for scope?
5. **Metered-interface detection:** rely on NetworkManager's metered flag vs manual labeling in MVP?
6. **Post-1.0 firewall scope:** confirm the step order in §6.3 after V1.0 feedback.

---

## 11. MVP Definition of Done (release criteria)

1. All §6.1 items implemented and QA'd on Arch/CachyOS, Ubuntu LTS (deb), Fedora (rpm).
2. Accounting accuracy validated in automated harness against interface counters (±5%).
3. Daemon passes hardening review (systemd sandbox active, localhost-only API, token auth).
4. Onboarding flow completes in < 3 min on a clean VM for all 3 package paths.
5. Degraded states (no daemon, no eBPF, permission denied) each have a tested UX path.
6. Pro license validation works offline; free/Pro gating verified.
7. Uninstall removes service; data removal tested.
8. Zero known P1/P2 bugs; crash-free rate ≥ 99.5% in beta.

---

## 12. Post-1.0 Roadmap (Firewall Phase-In — per product decision)

> Monitor-only until V1.0 is stable. Then add firewall capability step by step, reusing the eBPF foundation (observe → enforce), the local API, and the settings/notifications scaffolding — mirroring the reference architecture's proven separation (root engine, user-context UI, localhost API), without importing its Go/Angular stack.

- **1.1:** Rule preview engine (read-only "would block" analysis) + per-app block/allow rules behind an explicit opt-in.
- **1.2:** Verdict prompts, scope rules (localhost/LAN/Internet/P2P/Inbound), endpoint rules (domain/IP/Country).
- **1.3:** Secure DNS routing (DoT/DoH) + rebinding protection, DNS interception UX.
- **2.0:** Filter-list ecosystem (curated block lists + custom lists), pro-tier alignment revisited.

---

*End of PRD. No code was written as part of this document; §5 defines contracts only.*
