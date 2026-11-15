# env-vars.md — Environment Variables Mirror

**Binding source:** `../ARCHITECTURE.md` §8.3. This list is **closed** — adding a variable requires updating ARCHITECTURE.md §8.3 first (AGENTS.md §5). **Version:** 1.1 (2026-11-15).

| Var | Consumer | Default | Purpose |
|---|---|---|---|
| `NETWATCH_DATA_DIR` | netwatchd | `/var/lib/netwatch-pro` | DB + state directory (systemd sets it) |
| `NETWATCH_LOG_DIR` | netwatchd | `<data>/log` | Log output |
| `NETWATCH_LOG_LEVEL` | netwatchd | `info` | `error/warn/info/debug/trace` |
| `NETWATCH_API_ADDR` | netwatchd | `127.0.0.1:7897` | Loopback-only API bind address |
| `NETWATCH_API_TOKEN_FILE` | netwatchd (development/test only) | unset → `/var/lib/netwatch-pro/api-token` | Optional non-production token-file override. Packaged production daemon always writes `/var/lib/netwatch-pro/api-token`; helper path is fixed and never configurable. |
| `NETWATCH_RETENTION_DAYS` | netwatchd | `90` | Default retention; user setting overrides (Pro > 90) |
| `NETWATCH_DISABLE_EBPF` | netwatchd | unset | Diagnostic switch: run with empty collector (degraded chip) |
| `NETWATCH_DEV_DAEMON_ADDR` | UI (dev only) | unset | Point UI at a dev daemon; never read in packaged builds |

Secrets: the per-boot API token is the only secret — auto-generated, never user-set, never logged. The production token path is canonical and fixed; do not add this override to packaged systemd units.

Dev template: `.env.example` (repo root) mirrors this list for local runs; copy it to `.env` only if you need to override defaults. Nothing reads `.env` yet — the daemon lands in Phase 1 (PLAN.md P1.1).
