# env-vars.md — Environment Variables Mirror

**Binding source:** `../ARCHITECTURE.md` §8.3. This list is **closed** — adding a variable requires updating ARCHITECTURE.md §8.3 first (AGENTS.md §5). **Version:** 1.2 (2026-11-19). Phase A (live throughput) adds no environment variables: the 1 Hz rate task uses the existing interface exclusion list and the `speed_unit` setting.

| Var | Consumer | Default | Purpose |
|---|---|---|---|
| `NETWATCH_DATA_DIR` | netwatchd | `/var/lib/netwatch-pro` | DB + state directory (systemd sets it) |
| `NETWATCH_LOG_DIR` | netwatchd | `<data>/log` | Log output |
| `NETWATCH_LOG_LEVEL` | netwatchd | `info` | `error/warn/info/debug/trace` |
| `NETWATCH_API_SOCKET` | netwatchd + UI | `/run/netwatch-pro/api.sock` | Unix domain socket path; peer-credential UID allowlist authorization |
| `NETWATCH_API_GID` | netwatchd | `SUDO_GID` if set, else root GID | Group id allowed to connect; socket is created `0660` owned by this group |
| `NETWATCH_RETENTION_DAYS` | netwatchd | `90` | Default retention; user setting overrides (Pro > 90) |
| `NETWATCH_DISABLE_EBPF` | netwatchd | unset | Diagnostic switch: run with empty collector (degraded chip) |
| `NETWATCH_DEV_DAEMON_ADDR` | UI (dev only) | unset | Point UI at a dev daemon socket; never read in packaged builds |

Removed: `NETWATCH_API_ADDR`, `NETWATCH_API_TOKEN_FILE` (Unix socket + peer-credential replaces loopback TCP + bearer token).

Secrets: none at rest — the per-boot API token is retired; authorization is via the root-owned allowed-UID list (not a secret value).

Dev template: `.env.example` (repo root) mirrors this list for local runs; copy it to `.env` only if you need to override defaults. Nothing reads `.env` yet — the daemon lands in Phase 1 (PLAN.md P1.1).
