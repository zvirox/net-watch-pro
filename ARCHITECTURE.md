# Net Watch Pro — ARCHITECTURE.md (Technical Architecture)

**Version:** 1.0 · **Date:** 2026-11-14  
**Source of truth:** `PRD.md` (scope, contracts, non-goals) · `DESIGN.md` (UI system) · `REAPM.md` (reference teardown)  
**Rule:** This file defines structure and contracts only. **No application code is written as part of this document.**

> **Strict rules:** Every implementation decision must trace to this file. Stack, folder layout, data flow, naming, and env vars below are binding. Deviations require updating this file first (version bump), like DESIGN.md §11.

---

## 1. Architecture Principles

1. **Two processes, one boundary** — a root systemd daemon (`netwatchd`) that touches the kernel, and an unprivileged UI process (Tauri 2). They talk only over a localhost, token-authenticated WebSocket/HTTP API. This mirrors the proven separation in `REAPM.md` §3/§12 (core engine ≠ UI; UI stopping never stops monitoring).
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
| Database | **SQLite (WAL)** | Single-file, zero-admin, durable local history at 90+ days; the most documented embedded DB that exists. |
| DB access (daemon) | **sqlx (Rust)** | Compile-time-checked SQL with plain SQLite — no ORM magic, migrations built in. |
| Distribution | **AUR + .deb repo + .rpm repo** | The package manager is the "hosting": users install/update with tools they already trust (PRD §6.1 #10). |
| Hosting | **None (local-only product)** | Everything runs on-device per PRD non-goals; repos + offline-signed license keys replace any server dependency. |
| CI | **GitHub Actions + Earthly-style recipes** | One build definition produces the Linux binaries + packages and runs the 3-distro test matrix (PRD DoD). |

Explicitly **not** used (avoid over-engineering): Kubernetes/any server, GraphQL, gRPC (local WS + JSON is enough), Redis/queues (Tokio channels suffice), ORM entities, micro-frontends, Electron.

---

## 3. System Components

| Component | Runtime | Privilege | Responsibility |
|---|---|---|---|
| `netwatchd` | Rust/Tokio systemd service | root (bounded caps: `CAP_BPF`, `CAP_PERFMON`, `CAP_NET_ADMIN`) | Loads Aya programs, attributes traffic, writes SQLite, serves localhost WS/HTTP API, pruning, health endpoint |
| eBPF layer | kernel (CO-RE, BTF) | — | Per-socket/per-cgroup byte + packet counters, process identity metadata; pushed to daemon via ring buffer |
| UI process | Tauri 2 + Svelte 5 | user | All screens per DESIGN.md §8; connects to daemon WS; tray; no root ever |
| SQLite DB | file (WAL) | root-owned | Connections history, usage rollups, app registry, settings cache, schema migrations |
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
│   ├── netwatch-ebpf/               # eBPF programs (no_std, aya-bpf) — compiled to BPF object
│   │   ├── src/main.rs              # program entrypoints (socket/cgroup hooks)
│   │   └── Cargo.toml
│   ├── netwatch-core/               # daemon domain logic (no I/O deps): accounting, attribution, retention, rollups
│   │   ├── src/lib.rs
│   │   └── src/{accounting,attribution,retention}.rs
│   ├── netwatchd/                   # daemon binary: composition root
│   │   ├── src/main.rs
│   │   ├── src/{ebpf_loader,db,api,health,config}.rs
│   │   ├── src/db/migrations/*.sql  # embedded by sqlx
│   │   └── src/bin/                 # CLI verbs: --export-config, --recover-fw (future)
│   └── netwatch-api/                # API server: WS routes, auth token, request/validation types
│       └── src/lib.rs
│
├── app/                             # UI workspace (npm)
│   ├── package.json · vite.config.ts · tsconfig.json
│   ├── eslint.config.js · .prettierrc
│   ├── src-tauri/                   # Tauri 2 shell (Rust, thin)
│   │   ├── Cargo.toml · tauri.conf.json
│   │   ├── capabilities/            # Tauri permission grants (least privilege)
│   │   └── src/{main.rs,tray,window_state,daemon_link}.rs
│   └── src/                         # Svelte 5 frontend
│       ├── main.ts · App.svelte
│       ├── lib/
│       │   ├── api/                 # WS client, typed endpoint wrappers, reconnect logic
│       │   ├── stores/              # runes-based shared state (§7)
│       │   ├── components/          # shadcn-svelte vendored + adapted per DESIGN.md §7
│       │   │   ├── ui/              # primitives (button, input, table, toast, …)
│       │   │   └── charts/          # ECharts wrappers (theme-registered)
│       │   ├── theme/               # token CSS (DESIGN.md §3–6 generated vars)
│       │   ├── screens/             # one folder per DESIGN.md §8 page
│       │   │   ├── onboarding/ · dashboard/ · live/ · app-detail/
│       │   │   ├── history/ · settings/ · upgrade/ · diagnostics/
│       │   └── utils/               # formatters (bytes/rates per DESIGN.md §4.3)
│       └── tests/
│
├── xtask/                           # build automation crate: BPF compile, package, release
│
├── packaging/
│   ├── arch/PKGBUILD
│   ├── deb/                         # debian control + repo tooling
│   ├── rpm/                         # spec file + repo tooling
│   ├── systemd/netwatchd.service    # hardened unit (PRD §5 contract)
│   └── licenses/                    # offline license-key public key + verifier
│
├── docs/
│   ├── api-contract.md              # WS/HTTP API: routes, DTOs, error codes (§6 binding)
│   ├── db-schema.md                 # SQLite tables + migration policy
│   └── env-vars.md                  # mirrors §9.3
│
└── tests/
    ├── integration/                 # daemon<->db<->api on 3 distro containers
    ├── accuracy/                    # accounting vs interface counters harness (PRD ±5%)
    └── fixtures/                    # seeded sample data (demo mode, PRD OQ4)
```

Rules: no shared code by copy — shared types live in `netwatch-common`. The Tauri crate stays thin (window/tray/link only); all logic is daemon-side. `docs/api-contract.md` and `docs/db-schema.md` are binding and versioned like this file.

---

## 5. Data Flow

### 5.1 Runtime flow (happy path)

```
 Kernel                     netwatchd (root, Tokio)                      UI (user, Tauri 2 + Svelte 5)
┌──────────────┐  ring    ┌──────────────────────────────┐  WS/JSON  ┌────────────────────────────┐
│ Aya eBPF     │ buffer   │ ebpf_loader → attribution →  │ ────────▶ │ WS client → stores (§7) →  │
│ socket/cgrp  │ ─events▶ │ accounting → batch writer    │           │ screens (DESIGN.md §8)     │
│ counters     │ (1s flush)│  ↓ batched INSERT          │ ◀───────  │ actions: subscribe, query, │
└──────────────┘          │  ↓                    SQLite │  commands │ settings, export (Pro)     │
  /proc + procfs ◀────────┤ pruner / rollups    (WAL)    │           └────────────────────────────┘
  process enrichment      │ api server (127.0.0.1) ──────┼── auth ──▶ token file handshake (§6.2)
                          │ health endpoint              │
                          └──────────────────────────────┘
```

1. **Kernel → daemon:** eBPF maps accumulate per-socket bytes (up/down, packets); a ring buffer wakes the daemon; the daemon flushes aggregated deltas every ~1 s (PRD "sub-second refresh" satisfied at the UI refresh layer, not by flooding).
2. **Enrichment:** new socket identities are enriched from `/proc` (PID → exe, name, icon hint) — same proven pattern as `REAPM.md` §10 step 2–3; results cached in the app registry table.
3. **Persist:** writer batches inserts into `connections` + updates rollup tables; retention pruner runs on schedule (90-day default, configurable).
4. **Serve:** API server streams live snapshots/updates over WS; history queries are plain HTTP GET with pagination.
5. **UI:** Svelte stores render per DESIGN.md; every screen subscribes only to what it shows.

### 5.2 Third-party services

**None at runtime.** The app's own traffic is visible in its own Live view (PRD "dogfooding zero-outbound" metric). At build/install time only: package repos fetch artifacts; the offline license verifier uses an embedded public key — no server roundtrip in MVP.

### 5.3 API contract (summary — details in `docs/api-contract.md`)

- Base: `ws://127.0.0.1:<port>/api/v1` (HTTP GET for queries on the same port). Loopback bind only.
- Auth: daemon writes a per-boot random token to a root-only-readable file; the UI reads it via the Tauri shell's one-shot privileged helper at link time — the UI process never gets broader file access.
- Channels (WS): `live.connections` (snapshot + delta stream), `status` (daemon/eBPF/recording state → DESIGN.md chip), `usage.totals`. Commands: `subscribe`, `unsubscribe`, `history.query`, `export.request` (Pro-gated server-side, not just UI-gated), `settings.get/set`, `diagnostics.dump`.
- All payloads are versioned JSON; unknown fields ignored (forward compatible). Error payloads carry stable string codes (§8.2).

### 5.4 Database model (tables only — DDL lives in `docs/db-schema.md`)

| Table | Purpose | Growth |
|---|---|---|
| `connections` | one row per connection episode: app id, pid snapshot, proto, local/remote, bytes up/down, first/last seen | high → pruned/compacted |
| `usage_rollup_hour` / `usage_rollup_day` | per-app aggregated bytes after compaction | bounded |
| `apps` | app registry: name, icon path, first/last seen, attribution confidence | small |
| `interfaces` | known interfaces, metered flag (PRD OQ5) | tiny |
| `settings` | daemon-side settings mirror + schema version | tiny |
| `schema_migrations` | sqlx migration ledger | static |

Policy: raw `connections` older than the raw-window (7 days) is compacted into `usage_rollup_hour`; rollups serve History beyond that (PRD 90-day default; Pro up to 1 year). WAL mode, `synchronous=NORMAL`.

---

## 6. State Management Approach

### 6.1 UI (Svelte 5)

- **Runes are the store layer.** No external state library. Three state tiers:
  - **Connection state** (`lib/stores/daemon.svelte.ts`): WS link, auth, chip state per DESIGN.md §7.5. Owns reconnect with capped backoff (target: UI auto-recovers ≤ 5 s per PRD metric).
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
| SQLite tables/columns | snake_case, plural tables | `usage_rollup_hour` |
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
| `NETWATCH_API_ADDR` | netwatchd | `127.0.0.1:7897` | Loopback-only API bind address |
| `NETWATCH_API_TOKEN_FILE` | netwatchd | `<data>/api-token` | Where the per-boot token is written (mode 0600) |
| `NETWATCH_RETENTION_DAYS` | netwatchd | `90` | Default retention; user setting overrides (Pro > 90) |
| `NETWATCH_DISABLE_EBPF` | netwatchd | unset | Diagnostic switch: run with empty collector (degraded chip) |
| `NETWATCH_DEV_DAEMON_ADDR` | UI (dev only) | unset | Point the UI at a dev daemon; never read in packaged builds |

That is the entire list. Anything else must be added here first. Secrets: only the per-boot token (auto-generated, never user-set).

---

## 9. Scalability & Performance Notes

**Product scale is fixed (single desktop user); "scale" means data volume and kernel efficiency, not multi-tenancy.**

- **Kernel side:** counting happens in eBPF maps (aggregation in-kernel); the ring buffer carries deltas, not packets. Budget: ring buffer ≤ 1 MiB, drop-oldest on overflow; dropped-event counter exposed in Diagnostics (honest-status principle, DESIGN.md §1).
- **Daemon CPU/memory (PRD targets: ≤ 3% CPU, ≤ 150 MB RSS p95):** 1 s aggregation flush amortizes work; `/proc` reads only for *new* identities (cached in `apps`); SQLite writes batched (≤ 1 tx/s); no per-packet allocations.
- **SQLite growth (PRD R4):** WAL + `synchronous=NORMAL`; raw connections compacted to hourly rollups after 7 days; indexes only on query paths (app id, last seen, remote); `PRAGMA optimize` on clean shutdown; pruning runs off-peak schedule. Full-1-year Pro retention estimated < 1.5 GB on desktop-usage workloads — validated in the accuracy harness.
- **UI rendering:** live table virtualized (> 500 rows, DESIGN.md §7.6); rates update in place — no full-list re-render per tick; ECharts decimates time series above ~2 k points and re-renders on container resize only; stores batch WS deltas into one microtask flush.
- **Startup:** daemon ready < 2 s on supported kernels (eBPF attach + DB open + API bind); UI shows honest "Connecting…" until first snapshot rather than blocking splash.
- **Failure containment:** a dead collector must not corrupt DB state (writes transactional), must not wedge the API (tasks isolated by channel boundaries), and must not stall the UI (all reads time out with visible error states).
- **Future-proofing for firewall phase-in (PRD §12):** the enforcement path is an *additional* eBPF program + verdict task in the same slots defined here; the API gains command channels, not a new transport. No architectural rework is expected between 1.0 and 2.0.

---

## 10. Governance

Same model as DESIGN.md §11: this file is binding; changes require a version bump and a short rationale in the PR that updates it. `docs/api-contract.md` and `docs/db-schema.md` extend (never contradict) §5 and are versioned with it.

---

*End of ARCHITECTURE.md. No application code was written as part of this document — implementation follows these contracts.*
