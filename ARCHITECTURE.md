# Net Watch Pro — ARCHITECTURE.md (Technical Architecture)

**Version:** 1.11 · **Date:** 2026-11-21
**Source of truth:** `PRD.md` (scope, contracts, non-goals) · `DESIGN.md` (UI system) · `REAPM.md` (reference teardown) · `docs/decisions.md` (owner decisions D1–D7)

**Version history:** 1.3 (2026-11-15): apply plan-fix findings — Unix-socket + peer-credential transport (F-06/D1a), cgroup-first app identity (F-05/D5a), interface-counter sampler + Unattributed row (F-03/D3a), 15-minute rollup grain + timezone/cycle (F-04/D4a), daily endpoint rollups (F-02/D2a), in-memory episode state + checkpoint cadence (F-09), VM-based kernel matrix not containers (F-08), separate netwatch-ebpf build (F-01/F-11), [HYPOTHESIS] tags on unmeasured kernel claims. 1.4 (2026-11-19): Phase A of `docs/LIVE-THROUGHPUT-PLAN.md` — 1 Hz rate engine and `net.rates` stream beside the existing 15 s history sampler, `speed_unit` setting, and honest `no_collector` status. 1.5 (2026-11-19): 60-sample in-memory rate history for the last-hour view (lost on restart, `in_memory:true`) and optional Wi-Fi signal/link fields on U3 from local sysfs (owner L-OQ1/L-OQ4). 1.6 (2026-11-20): Phase C eBPF collector — `NETWATCH_EBPF_OBJECT` runtime object path; attach reads per-socket bytes by cgroup id (`bpf_skb_cgroup_id`, spike-verified), not `current`. 1.7 (2026-11-20): Phase D — new `screens/apps/` and `stores/apps.svelte.ts`, shared `stores/range.svelte.ts`, `range_default` setting, and `range` query parameter on the usage/apps APIs. 1.8 (2026-11-20): self-contained eBPF collector — `netwatchd/build.rs` embeds the object produced by `cargo run -p xtask -- bpf` via `aya::include_bytes_aligned!`; no runtime object path in packaged builds, `NETWATCH_EBPF_OBJECT` retained as a dev override. 1.9 (2026-11-21): per-socket `FLOWS` map in the existing cgroup_skb programs feeds `EpisodeEvent`s into the live service, so `live.connections` and `connections` get real rows. IPv4 only; PID is best-effort from the cgroup scope name (0 = unknown); close is by idle timeout because measured hooks have no reliable close callback. 1.11 (2026-11-21): Phase 10 hardening — seeded 1-year soak profile defined below; accuracy/perf harnesses; CI container smoke replaces the VM matrix by owner decision (labelled as such, does not satisfy DoD #1). 1.10 (2026-11-21): owner-only dev license testing — `NETWATCH_LICENSE_PUBLIC_KEY` overrides the embedded verifier key in debug builds only; `cargo run -p xtask -- license-dev-key` mints an ephemeral keypair and a signed NWP1 key.
**Rule:** This file defines structure and contracts only. **No application code is written as part of this document.**

> **Strict rules:** Every implementation decision must trace to this file. Stack, folder layout, data flow, naming, and env vars below are binding. Deviations require updating this file first (version bump), like DESIGN.md §11.

---

## 1. Architecture Principles

1. **Two processes, one boundary** — a root systemd daemon (`netwatchd`) that touches the kernel, and an unprivileged UI process (Tauri 2). They talk only over a **Unix domain socket with peer-credential (`SO_PEERCRED`) authorization against a root-owned allowed-UID list**. This mirrors the proven separation in `REAPM.md` §3/§12 (core engine ≠ UI; UI stopping never stops monitoring). One admin prompt at enable-time during onboarding; no per-launch prompt (D1a).
2. **Zero outbound by default** — the product makes no network calls to third-party services. No cloud, no telemetry, no accounts. The only optional outbound is the user-triggered license purchase URL and (post-MVP decision, PRD OQ3) update metadata checks via the OS package manager — not the app.
3. **Observe today, enforce tomorrow** — the eBPF layer is a program slot in the daemon, not hardwired logic. The post-1.0 firewall phase-in (PRD §12) swaps/adds programs and adds a verdict pipeline; nothing else in the architecture changes.
4. **SQLite is the single source of persisted truth** — one database file, WAL mode, schema migrations bundled with the daemon. No caches, no second store.
5. **Simple, popular, boring tools** — no custom frameworks, no ORMs with magic, no micro-services inside a desktop app. One workspace, few crates, one SQL database.

---

## 2. Recommended Tech Stack (mandated + rationale)

| Layer | Choice | One-line reason |
|---|---|---|
| Desktop shell | **Tauri 2** | Native window/tray/notifications with a tiny binary and a Rust core we can keep unprivileged by design. |
| Frontend | **Svelte 5** | Runes give fine-grained reactivity for high-frequency live tables without a virtual-DOM tax. |
| Components | **shadcn-svelte** | Accessible, vendorable primitives we style strictly via DESIGN.md tokens — no theme lock-in. |
| Charts | **ECharts** | Battle-tested canvas charts that handle streaming/decimated time series and theming from our tokens. |
| Daemon backend | **Rust + Tokio** | One language for eBPF loaders, async IPC, and DB I/O; memory safety in a root process is non-negotiable. |
| Monitoring | **Aya eBPF** | Pure-Rust CO-RE eBPF: socket/cgroup hooks for per-process byte accounting without packet rewriting (monitor-only MVP). |
| IPC transport | **Unix domain socket** | Peer-credential (`SO_PEERCRED`) UID allowlist; replaces loopback TCP + bearer token (D1a). No CORS/Origin/CSRF surface. |
| Database | **SQLite (WAL)** | Single-file, zero-admin, durable local history at 90+ days; the most documented embedded DB that exists. |
| DB access (daemon) | **sqlx (Rust)** | Compile-time-checked SQL with plain SQLite — no ORM magic, migrations built in. |
| Distribution | **AUR + .deb repo + .rpm repo** | The package manager is the "hosting": users install/update with tools they already trust (PRD §6.1 #10). |
| Hosting | **None (local-only product)** | Everything runs on-device per PRD non-goals; repos + offline-signed license keys replace any server dependency. |
| CI | **GitHub Actions + Earthly-style recipes** | One build definition produces the Linux binaries + packages and runs the 3-distro test matrix on **VMs** (PRD DoD; containers share the host kernel — F-08). |

Explicitly **not** used (avoid over-engineering): Kubernetes/any server, GraphQL, gRPC (local WS + JSON is enough), Redis/queues (Tokio channels suffice), ORM entities, micro-frontends, Electron.

---

## 3. System Components

| Component | Runtime | Privilege | Responsibility |
|---|---|---|---|
| `netwatchd` | Rust/Tokio systemd service | root (bounded caps: `CAP_BPF`, `CAP_PERFMON`, `CAP_NET_ADMIN`) | Loads Aya programs, attributes traffic (cgroup-first identity, `/proc` fallback), writes SQLite, serves Unix-socket JSON API, 15 s interface-counter sampler, 1 Hz rate sampler for live PC speed, pruning, health endpoint |
| eBPF layer | kernel (CO-RE, BTF) | — | Per-socket/per-cgroup byte + packet counters, process identity metadata; pushed to daemon via ring buffer |
| UI process | Tauri 2 + Svelte 5 | user | All screens per DESIGN.md §8; connects to daemon Unix socket; tray; no root ever |
| SQLite DB | file (WAL) | root-owned | Connections history, usage rollups (hour/day/15m + endpoint-day), app registry, settings cache, schema migrations |
| IPC socket | `/run/netwatch-pro/api.sock` | root-owned dir, socket `0660` owned by `netwatch` group | Peer-credential check vs allowed-UID list at `/etc/netwatch-pro/allowed-uids` (one numeric UID per line, root-owned `0644`); written by the one-time onboarding pkexec step, none per launch |
| Packaging | PKGBUILD / deb / rpm + systemd unit | installer | Registers hardened unit (ProtectSystem, PrivateTmp, RestrictAddressFamilies), installs DB dir `/var/lib/netwatch-pro` |

---

## 4. Folder & File Structure

Single monorepo, Cargo workspace for the Rust side, npm workspace for the UI.

```
net-watch-pro/
├── README.md
├── ARCHITECTURE.md                  # this file (binding)
├── PRD.md · DESIGN.md · REAPM.md    # sibling source-of-truth docs
├── Cargo.toml                       # Cargo workspace root
├── rust-toolchain.toml              # pinned stable Rust
├── deny.toml · clippy.toml          # cargo-deny / clippy config
├── .editorconfig · .gitignore
│
├── crates/                          # Rust workspace members
│   ├── netwatch-common/             # shared types: events, IDs, error codes, API DTOs
│   │   └── src/lib.rs
│   ├── netwatch-ebpf/               # eBPF programs (no_std, aya-bpf) — compiled to BPF object; SEPARATE build (nightly + bpf-linker), NOT a Cargo workspace member (F-01/F-11)
│   │   ├── src/main.rs              # program entrypoints (socket/cgroup hooks)
│   │   └── Cargo.toml
│   ├── netwatch-core/               # daemon domain logic (no I/O deps): accounting, attribution, retention, rollups
│   │   ├── src/lib.rs
│   │   └── src/{accounting,attribution,retention,rates,settings}.rs
│   ├── netwatchd/                   # daemon binary: composition root
│   │   ├── build.rs                 # copies prebuilt BPF object into OUT_DIR for embedding (P8.9)
│   │   ├── src/main.rs
│   │   ├── src/{ebpf_loader,db,api,health,config,rates,sampler,pruner,service}.rs
│   │   ├── src/db/migrations/*.sql  # embedded by sqlx
│   │   └── src/bin/                 # CLI verbs: --export-config, --recover-fw (future)
│   ├── netwatch-api/                # API server: Unix-socket JSON routes, peer-cred auth, request/validation types
│   │   └── src/lib.rs
│   └── netwatch-token-helper/       # REMOVED (D1a): Unix-socket + peer-credential replaces token helper; deleted after owner approval
│       └── src/main.rs              # (placeholder pending deletion)
│
├── app/                             # UI workspace (npm)
│   ├── package.json · vite.config.ts · tsconfig.json
│   ├── eslint.config.js · .prettierrc
│   ├── src-tauri/                   # Tauri 2 shell (Rust, thin)
│   │   ├── Cargo.toml · tauri.conf.json
│   │   ├── capabilities/            # Tauri permission grants (least privilege)
│   │   └── src/{main.rs,tray,window_state,daemon_link}.rs   # token_link.rs removed (D1a)
│   └── src/                         # Svelte 5 frontend
│       ├── main.ts · App.svelte
│       ├── lib/
│       │   ├── api/                 # Unix-socket client, typed endpoint wrappers, reconnect logic
│       │   ├── stores/              # runes-based shared state (§7)
│       │   │   ├── range.svelte.ts  # shared range preset + auto bucket (Phase D)
│       │   │   └── apps.svelte.ts   # A1 ranking screen data (Phase D)
│       │   ├── components/          # shadcn-svelte vendored + adapted per DESIGN.md §7
│       │   │   ├── ui/              # primitives (button, input, table, toast, …)
│       │   │   └── charts/          # ECharts wrappers (theme-registered)
│       │   ├── theme/               # token CSS (DESIGN.md §3–6 generated vars)
│       │   ├── screens/             # one folder per DESIGN.md §8 page
│       │   │   ├── onboarding/ · dashboard/ · live/ · apps/ · app-detail/
│       │   │   ├── history/ · settings/ · upgrade/ · diagnostics/
│       │   └── utils/               # formatters (bytes/rates per DESIGN.md §4.3)
│       └── tests/
│
├── spikes/                          # throwaway spike code (outside Cargo workspace)
│   └── ebpf-accounting/             # eBPF accounting spike (F-01 Part A); own Cargo.toml/rust-toolchain.toml
│
├── xtask/                           # build automation crate: BPF compile, package, release
│
├── packaging/
│   ├── arch/PKGBUILD
│   ├── deb/                         # debian control + repo tooling
│   ├── rpm/                         # spec file + repo tooling
│   ├── systemd/netwatchd.service    # hardened release unit (PRD §5 contract)
│   │   └── netwatchd-dev.service     # unhardened local dev unit (P1.8)
│   ├── licenses/                    # offline license-key public key + verifier
│   └── tests/                       # package maintainer-script checks (P8.7)
│   # packaging/polkit/ REMOVED (D1a): no per-launch Polkit token helper
│
├── docs/
│   ├── api-contract.md              # WS/HTTP API: routes, DTOs, error codes (§6 binding)
│   ├── db-schema.md                 # SQLite tables + migration policy
│   ├── env-vars.md                  # mirrors §8.3
│   └── spike-ebpf-report.md         # spike report (F-01 Part A deliverable 2)
│
└── tests/
    ├── integration/                 # daemon<->db<->api on **VMs** (F-08: containers share host kernel, cannot test kernel variance)
    ├── accuracy/                    # accounting vs interface counters harness (PRD accuracy: totals equal kernel counters, ≥95% attributed)
    ├── a11y/                        # stdlib contrast check for DESIGN.md §9.1 token pairs (P9.4)
    └── fixtures/                    # seeded sample data (demo mode, PRD OQ4)
```

Rules: no shared code by copy — shared types live in `netwatch-common`. The Tauri crate stays thin (window/tray/link only); all logic is daemon-side. `docs/api-contract.md` and `docs/db-schema.md` are binding and versioned like this file.

---

## 5. Data Flow

### 5.1 Runtime flow (happy path)

```
 Kernel                     netwatchd (root, Tokio)                      UI (user, Tauri 2 + Svelte 5)
┌──────────────┐  ring    ┌──────────────────────────────┐  UDS/JSON  ┌────────────────────────────┐
│ Aya eBPF     │ buffer   │ ebpf_loader → attribution →  │ ────────▶ │ socket client → stores →   │
│ socket/cgrp  │ ─events▶ │ accounting → batch writer    │           │ screens (DESIGN.md §8)     │
│ counters     │ (1s flush)│ ↓ batched INSERT             │ ◀───────  │ actions: subscribe, query, │
└──────────────┘          │ ↓                    SQLite  │  commands │ settings, export (Pro)     │
  /proc + procfs ◀────────┤ pruner / rollups    (WAL)    │  + peer-cred│ └────────────────────────────┘
  process enrichment      │ api server (UDS) ────────────┼── auth ──▶ allowed-UID list (§6.2)   │
                          │ health endpoint              │                                      │
                          │ interface-counter sampler    │                                      │
                          └──────────────────────────────┘                                      │
                                                            ┌────────────────────────────┐      │
                                                            │ Unix socket: peer-cred (    │◄─────┘
                                                            │ SO_PEERCRED UID allowlist)   │
                                                            └────────────────────────────┘
```

1. **Kernel → daemon:** eBPF maps accumulate per-socket bytes (up/down, packets); a ring buffer wakes the daemon; the daemon flushes aggregated deltas every ~1 s (PRD "sub-second refresh" satisfied at the UI refresh layer, not by flooding). **[HYPOTHESIS]** flush cadence and map sizes to be confirmed by spike Q1/Q8.
2. **Enrichment:** new socket identities are captured **at exec/connect time in-kernel** (e.g. `sched_process_exec` tracepoint) and resolved via cgroupfs path → cgroup identity (`systemd app-*.scope`, Flatpak, Snap); `/proc/<pid>/exe` reads are a **fallback only**, not the primary source (D5a). **[HYPOTHESIS]** exec-time capture reliability unmeasured until spike Q4/Q5; reading other users' `/proc` may need capabilities beyond the bounded set (Appendix B, item 5).
3. **Persist:** open-episode state is kept **in memory**; persisted on episode close and at a checkpoint interval (default ~60 s — crash-loss window ≤ one interval). Writer batches inserts into `connections` + updates rollup tables; retention pruner runs on schedule (90-day default, configurable). **[HYPOTHESIS]** per-second persistence of hundreds of open connections is materially more expensive than checkpoint cadence (Appendix B, item 10 — to be measured in spike Q10).
4. **Interface-counter sampler:** a daemon task reads per-interface kernel counters (`/sys/class/net/*/statistics` or netlink stats) for the headline total; `lo` excluded; tunnel interfaces (VPN/WireGuard) handled to avoid double-counting (PRD R6). A separate **1 Hz rate task** reads the same counters for `net.rates` live speed (Phase A); it uses measured elapsed time and EMA smoothing, and never writes SQLite. **[HYPOTHESIS]** header/ACK overhead is "a few percent" of payload (Appendix B, item 6).
5. **Serve:** API server streams live snapshots/updates over the Unix socket; history queries are plain requests with pagination.
6. **UI:** Svelte stores render per DESIGN.md; every screen subscribes only to what it shows.
7. **Implemented per-socket path (P2.2/P2.3, 2026-11-21):** the cgroup_skb programs also maintain a `FLOWS` map keyed by `bpf_get_socket_cookie` (measured-legal in cgroup_skb) with IPv4 tuple, cgroup id and absolute byte/packet counters. The daemon reads it at 1 Hz, resolves the cgroup to an app via `AppNames`, registers the app (`app_key = cgroup:<id>`) to obtain `app_id`, and emits `Open`/`Bytes`/`Close` `EpisodeEvent`s into `live::run`. Limitations, all honest: IPv6 flows are counted only in the cgroup rollups; `pid` is parsed best-effort from the cgroup scope name and is `0` (rendered "—") when unknown because every measured in-kernel source returned the softirq thread; `Close` is emitted after an idle timeout since no reliable close callback was measured. No new attach type or ring buffer is required.

### 5.1a Kernel collection design (contract — pending spike verification)

The eBPF collection mechanism is **not yet finalized**; it will be fixed by the time-boxed spike (Part A of `PLAN-FIX-INSTRUCTIONS.md`) and this section updated with `[VERIFIED: spike-ebpf-report §n]` tags afterward. The candidate designs below are ordered for comparison (accuracy, overhead, kernel floor, complexity, failure modes); do not pick on elegance.

| Design | Description | What is counted | Status |
|---|---|---|---|
| **Design 1 — cgroup_skb counters keyed by socket cookie** | Count bytes in `cgroup_skb` ingress/egress per socket cookie; fill cookie→(pid, cgroup) map at connect/accept time via cgroup `sock`/`sock_addr`/`sockops` hooks | L3 bytes (closer to interface counters) | `[HYPOTHESIS]` ingress runs in softirq context → `current` PID unreliable; hence the cookie map (Appendix B, item 1) |
| **Design 2 — function hooks** | kprobe/fentry on `tcp_sendmsg`, `tcp_cleanup_rbuf`/`tcp_recvmsg`, `udp_sendmsg`, `udp_recvmsg` with socket pointer + process context | Payload bytes, not wire bytes | `[HYPOTHESIS]` fentry has newer-kernel requirements on arm64 (Appendix B, item 7) |
| **Design 3 — hybrid** | Design 1 for byte counts; Design 2 only to fill identity where the cookie map misses | L3 bytes + identity | `[HYPOTHESIS]` unmeasured combined overhead |

**Contract elements to fix after the spike:**
- Hooks used and map types/sizes (LRU for per-socket state)
- Key fields (socket cookie or equivalent)
- Flush cadence and drop semantics (ring buffer ≤ 1 MiB, drop-oldest; dropped-event counter exposed in Diagnostics)
- Kernel floor per hook (not a single "5.10+ BTF" floor)
- Required capabilities (measured in spike Q6; bounded set to be confirmed)
- `[HYPOTHESIS]` cgroup-attached programs require cgroup v2 on all target distros (Appendix B, item 3)

See `docs/spike-ebpf-report.md` for the measurement tables, chosen design, and rejected designs.

### 5.1b Live rate history & Wi-Fi link fields (owner L-OQ1/L-OQ4)

- **In-memory rate history:** the 1 Hz rate task (§5.1 item 4) also feeds a
  bounded 60-entry ring (`VecDeque`, oldest dropped) at **one-minute**
  resolution: a newer sample in the current minute replaces the previous point,
  so 60 points cover the last hour. It is process memory only — never written
  to SQLite — and is served by
  `GET /usage/rate-history` with `in_memory:true` so the UI labels the
  last-hour view as lost-on-restart. Authenticated reads only; no new table or
  migration. Longer ranges keep using `usage_rollup_15m`.
- **Wi-Fi link fields:** U3 enriches each wireless interface row at request time
  by reading local sysfs only — `/sys/class/net/<iface>/wireless/link`
  (signal, dBm) and `/sys/class/net/<iface>/speed` (negotiated Mbps). The UI
  never touches sysfs (AGENTS.md §5). Reads are injectable-root for tests;
  missing/invalid values serialize as `null` and render as "—", never zero.
  No network call is made. **[HYPOTHESIS]** the file is read verbatim as dBm
  per the owner decision; on kernels where `wireless/link` is a legacy
  0–100 link-quality percentage instead, the UI label must be revisited after
  verification on the target hardware — do not present that guess as a
  measured contract.


**None at runtime.** The app's own traffic is visible in its own Live view (PRD "dogfooding zero-outbound" metric). At build/install time only: package repos fetch artifacts; the offline license verifier uses an embedded public key — no server roundtrip in MVP.

### 5.3 API contract (summary — details in `docs/api-contract.md`)

- Base: **Unix domain socket** `/run/netwatch-pro/api.sock` by default (`NETWATCH_API_SOCKET` env var). Versioned JSON over the socket; HTTP GET for queries on the same socket.
- Auth: **peer-credential (`SO_PEERCRED`)** check against a root-owned allowed-UID list; UI never requires root. Loopback TCP + bearer token removed (D1a). `NETWATCH_API_TOKEN_FILE` and the Polkit token helper are retired; the allowed-UID list is written during the one-time onboarding pkexec step.
- Channels (socket-based stream): `live.connections` (snapshot + delta stream), `status` (daemon/eBPF/recording state → DESIGN.md chip), `usage.totals`, `net.rates` (1 Hz PC speed + per-interface rates, snapshot on subscribe). `live.apps` is reserved for the accepted-collector phase and is not emitted yet. Commands: `subscribe`, `unsubscribe`, `history.query`, `export.request` (Pro-gated server-side), `settings.get/set`, `diagnostics.dump`.
- All payloads are versioned JSON; unknown fields ignored (forward compatible). Error payloads carry stable string codes (§8.2).

### 5.4 Database model (tables only — DDL lives in `docs/db-schema.md`)

| Table | Purpose | Growth |
|---|---|---|
| `connections` | one row per connection episode: app id, pid snapshot, proto, local/remote, bytes up/down, first/last seen | high → pruned/compacted |
| `usage_rollup_15m` | per-app aggregated bytes in 15-minute buckets (base grain); day/week/month boundaries computed at query time in configured timezone | bounded |
| `usage_endpoint_day` | per-app/day endpoint aggregates `(app_id, bucket_start, remote_ip, remote_port, protocol)` with cardinality cap (top N by bytes; rest folded into `other` row) | bounded (cap N) |
| `usage_interface_15m` | per-interface 15-minute counters for headline totals (source of truth for ISP-billed bytes); `lo` excluded | tiny |
| `apps` | app registry: name, `app_key`, `identity_kind`, icon path, first/last seen, attribution confidence | small |
| `interfaces` | known interfaces, metered flag (PRD OQ5) | tiny |
| `settings` | daemon-side settings mirror + schema version (includes `timezone`, `billing_cycle_day`) | tiny |
| `schema_migrations` | sqlx migration ledger | static |

Policy: raw `connections` older than the raw-window (7 days default) is compacted into `usage_rollup_15m`; `usage_endpoint_day` serves host/port/proto searchable history beyond the raw window (90 days free; up to 1 year Pro); `usage_interface_15m` feeds the headline total with an explicit "Unattributed / system" gap row (D3a). WAL mode, `synchronous=NORMAL`. **Size estimate** `< 1.5 GB` for 1-year Pro retention is `[HYPOTHESIS]` until the 1-year soak test (PRD P10.3); formula: rows/day × bytes/row × retention.

---

## 6. State Management Approach

### 6.1 UI (Svelte 5)

- **Runes are the store layer.** No external state library. Three state tiers:
  - **Connection state** (`lib/stores/daemon.svelte.ts`): socket link, auth, chip state per DESIGN.md §7.5. Owns reconnect with capped backoff (target: UI auto-recovers ≤ 5 s per PRD metric).
  - **Live data state** (`live.svelte.ts`, `usage.svelte.ts`): server-pushed data kept verbatim; stores expose *derived* sorted/filtered views so screens stay dumb.
  - **Local UI state** (`settings-ui.svelte.ts`): theme, sidebar collapse, frozen-snapshot flag — persisted via Tauri store, mirrors the reference app's `core:ui/v1` pattern (REAPM.md §8).
- **Server is the source of truth** — no optimistic writes, no client-side reconciliation of usage numbers; stale/detached states render honest UI (chip "Connecting…", banners per DESIGN.md §7.9) instead of cached guesses.
- Screens never touch the WS client directly; they read stores. Stores never import screens.

### 6.2 Daemon (Rust)

- Single process, Tokio multi-thread runtime; task-per-concern: `ebpf-loader`, `collector`, `writer`, `pruner`, `api`, `health`. Tasks communicate via Tokio mpsc channels with **bounded queues + drop-oldest on overflow** (backpressure policy §10).
- Settings live in SQLite; a reload broadcast task applies changes without restart (mirrors the reference registry behavior, REAPM.md §5).

---

## 7. Error-Handling Strategy

### 7.1 Daemon (Rust)

- `thiserror` typed errors per crate (`netwatch-core::Error`, `netwatch-api::ApiError`, …); `anyhow` only at the binary edge for logs. **No `unwrap()`/`expect()` in daemon paths outside startup**; worker tasks catch panics, log, and surface degraded status rather than dying.
- Startup is fail-fast with a distinct exit code per class (missing caps, BTF unavailable, DB unwritable) — systemd and Diagnostics (DESIGN.md §8.9) key off it.
- Every DB write is transactional; partial flushes are never persisted.

### 7.2 Error taxonomy (stable codes over the API)

`DAEMON_UNREACHABLE` · `AUTH_INVALID` · `EBPF_ATTACH_FAILED` · `EBPF_UNSUPPORTED_KERNEL` · `PERMISSION_DENIED` · `DB_ERROR` · `QUERY_INVALID` · `EXPORT_PRO_REQUIRED` · `LICENSE_INVALID` · `RATE_LIMITED`

### 7.3 UI mapping (to DESIGN.md states — never new patterns)

| Code | UI treatment |
|---|---|
| `DAEMON_UNREACHABLE` / `AUTH_INVALID` | chip "Stopped/Connecting" + daemon-stopped banner (§7.9) + Diagnostics entry |
| `EBPF_ATTACH_FAILED` / `EBPF_UNSUPPORTED_KERNEL` | Degraded chip + onboarding failure card / banner with guidance + retry |
| `PERMISSION_DENIED` | banner with "Start daemon" (pkexec) action |
| `DB_ERROR` | error toast + retry on the failed view only |
| `QUERY_INVALID` / `EXPORT_PRO_REQUIRED` / `LICENSE_INVALID` | inline input error (§7.2) or Upgrade modal (§8.8) |
| `RATE_LIMITED` | silent backoff; surfaced only in Diagnostics |

Retry policy: user-initiated retries everywhere except WS reconnect (automatic, capped exponential backoff 1 s → 16 s). Errors never leave the user on a blank screen: every screen implements loading/empty/error per DESIGN.md §7/§8.
---

## 8. Naming Conventions & Code Style

### 8.1 Naming

| Scope | Convention | Example |
|---|---|---|
| Rust crates | kebab-case | `netwatch-core` |
| Rust modules/files | snake_case | `accounting.rs` |
| Rust types | UpperCamelCase | `ConnectionEvent` |
| SQLite tables/columns | snake_case, plural tables | `usage_rollup_15m` |
| WS routes/channels | dot-namespaced kebab | `live.connections` |
| Env vars | `NETWATCH_` + UPPER_SNAKE | `NETWATCH_DATA_DIR` |
| TS files | kebab-case | `daemon-link.ts` |
| Svelte components | PascalCase.svelte | `StatusChip.svelte` |
| Stores | domain.svelte.ts, camelCase exports | `daemonStore` |
| CSS | tokens only (DESIGN.md §3) | `--nwp-primary` |
| API DTOs | camelCase JSON | `bytesUp` |
| Git | Conventional Commits | `feat(dashboard): …` |

### 8.2 Code style

- **Rust:** rustfmt defaults + clippy `-D warnings`; `#![deny(unsafe_code)]` everywhere except `netwatch-ebpf` and the Aya loader; MSRV pinned in `rust-toolchain.toml`; public crate APIs documented (`missing_docs` warn).
- **TS/Svelte:** ESLint + Prettier, strict TS (`noUncheckedIndexedAccess`), no `any`; Svelte 5 runes syntax only (no legacy `let` reactivity); components are presentational where stores exist.
- **SQL:** migrations only via sqlx `migrations/*.sql` (forward-only, never edit applied files); no business SQL outside `netwatch-core`/`netwatchd::db`.
- **Comments:** explain *why*, not what; every public daemon setting and API field cross-references its PRD/DESIGN section.
- **Reviews:** PR template checklist — "Does this follow DESIGN.md §7/§8 and ARCHITECTURE.md §4–§9?"

### 8.3 Environment variables (complete list)

| Var | Consumer | Default | Purpose |
|---|---|---|---|
| `NETWATCH_DATA_DIR` | netwatchd | `/var/lib/netwatch-pro` | DB + state directory (systemd sets it) |
| `NETWATCH_LOG_DIR` | netwatchd | `NETWATCH_DATA_DIR/log` | Log output |
| `NETWATCH_LOG_LEVEL` | netwatchd | `info` | `error/warn/info/debug/trace` |
| `NETWATCH_API_SOCKET` | netwatchd + UI | `/run/netwatch-pro/api.sock` | Unix domain socket path; peer-credential UID allowlist authorization |
| `NETWATCH_API_GID` | netwatchd | `SUDO_GID` if set, else root GID | Group id allowed to connect: socket is created `0660` owned by this group (desktop user's primary group in dev, service group in packages) |
| `NETWATCH_ALLOWED_UIDS_FILE` | netwatchd | `/etc/netwatch-pro/allowed-uids` | One UID per line (`#` comments). Peer-credential allowlist read on every connection; root, the daemon's own UID and `SUDO_UID` are always allowed. Missing file = no extra UIDs (fail closed) |
| `NETWATCH_RETENTION_DAYS` | netwatchd | `90` | Default retention; user setting overrides (Pro > 90) |
| `NETWATCH_DISABLE_EBPF` | netwatchd | unset | Diagnostic switch: run with empty collector (degraded chip) |
| `NETWATCH_EBPF_OBJECT` | netwatchd (dev only) | unset | Dev override: read the eBPF object from this path instead of the embedded bytes. Packaged builds need no object file; a missing override falls back to the embedded object. An explicitly bad override is a degraded state, never a startup failure |
| `NETWATCH_DEV_DAEMON_ADDR` | UI (dev only) | unset | Point the UI at a dev daemon socket; never read in packaged builds |
| `NETWATCH_LICENSE_PUBLIC_KEY` | netwatch-api (debug builds only) | unset | 64-char Ed25519 public-key hex overriding the embedded verifier for owner testing; ignored in release builds |

**Removed:** `NETWATCH_API_ADDR`, `NETWATCH_API_TOKEN_FILE` (D1a — Unix socket + peer-credential replaces loopback TCP + bearer token).

That is the entire list. Anything else must be added here first. Secrets: none at rest — the per-boot token is retired; authorization is via the root-owned allowed-UID list (D1a).

---

## 9. Scalability & Performance Notes

**Product scale is fixed (single desktop user); "scale" means data volume and kernel efficiency, not multi-tenancy.**

- **Kernel side:** counting happens in eBPF maps (aggregation in-kernel); the ring buffer carries deltas, not packets. Budget: ring buffer ≤ 1 MiB, drop-oldest on overflow; dropped-event counter exposed in Diagnostics (honest-status principle, DESIGN.md §1). **[HYPOTHESIS]** map sizes and drop behavior unmeasured until spike Q8.
- **Daemon CPU/memory (PRD targets: ≤ 3% CPU, ≤ 150 MB RSS p95):** open-episode state kept **in memory**, persisted on episode close + checkpoint interval (default ~60 s — crash-loss window ≤ one interval); SQLite writes batched from in-memory deltas (≤ 1 tx/s equivalent); `/proc` reads only for *new* identities (cached in `apps`); no per-packet allocations. Interface-counter sampler reads per-interface stats on a separate cadence for the headline total. **[HYPOTHESIS]** per-second persistence vs checkpoint cadence cost unmeasured until spike Q10 (Appendix B, item 10).
- **SQLite growth (PRD R4):** WAL + `synchronous=NORMAL`; raw connections compacted to 15-minute rollups after 7 days; `usage_endpoint_day` capped at top N per app/day; indexes only on query paths; `PRAGMA optimize` on clean shutdown; pruning runs off-peak schedule. Full-1-year Pro retention estimated < 1.5 GB on desktop-usage workloads — `[HYPOTHESIS]` until the 1-year soak test. Seeded soak profile (P10.3): 365 days × 40 apps × 96 fifteen-minute rollups (≈1.4 M rows) plus 365 days × 40 apps × 20 endpoint-day rows (≈292 k rows), with representative byte magnitudes; the harness asserts the resulting DB file stays under 1.5 GiB.
- **UI rendering:** live table virtualized (> 500 rows, DESIGN.md §7.6); rates update in place — no full-list re-render per tick; ECharts decimates time series above ~2 k points and re-renders on container resize only; stores batch socket deltas into one microtask flush.
- **Startup:** daemon ready < 2 s on supported kernels (eBPF attach + DB open + API bind); UI shows honest "Connecting…" until first snapshot rather than blocking splash.
- **Failure containment:** a dead collector must not corrupt DB state (writes transactional), must not wedge the API (tasks isolated by channel boundaries), and must not stall the UI (all reads time out with visible error states).
- **Future-proofing for firewall phase-in (PRD §12):** the enforcement path is an *additional* eBPF program + verdict task in the same slots defined here; the API gains command channels, not a new transport. No architectural rework is expected between 1.0 and 2.0.

---

## 10. Governance

Same model as DESIGN.md §11: this file is binding; changes require a version bump and a short rationale in the PR that updates it. `docs/api-contract.md` and `docs/db-schema.md` extend (never contradict) §5 and are versioned with it.

---

*End of ARCHITECTURE.md. No application code was written as part of this document — implementation follows these contracts.*
