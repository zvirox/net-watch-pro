# env-vars.md — Environment Variables Mirror

**Binding source:** `../ARCHITECTURE.md` §8.3. This list is **closed** — adding a variable requires updating ARCHITECTURE.md §8.3 first (AGENTS.md §5).

| Var | Consumer | Default | Purpose |
|---|---|---|---|
| `NETWATCH_DATA_DIR` | netwatchd | `/var/lib/netwatch-pro` | DB + state directory (systemd sets it) |
| `NETWATCH_LOG_DIR` | netwatchd | `<data>/log` | Log output |
| `NETWATCH_LOG_LEVEL` | netwatchd | `info` | `error/warn/info/debug/trace` |
| `NETWATCH_API_ADDR` | netwatchd | `127.0.0.1:7897` | Loopback-only API bind address |
| `NETWATCH_API_TOKEN_FILE` | netwatchd | `<data>/api-token` | Per-boot token file (mode 0600) |
| `NETWATCH_RETENTION_DAYS` | netwatchd | `90` | Default retention; user setting overrides (Pro > 90) |
| `NETWATCH_DISABLE_EBPF` | netwatchd | unset | Diagnostic switch: run with empty collector (degraded chip) |
| `NETWATCH_DEV_DAEMON_ADDR` | UI (dev only) | unset | Point UI at a dev daemon; never read in packaged builds |

Secrets: the per-boot API token is the only secret — auto-generated, never user-set, never logged.
