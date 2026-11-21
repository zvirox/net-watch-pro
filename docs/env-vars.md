# env-vars.md — Environment Variables Mirror

**Binding source:** `../ARCHITECTURE.md` §8.3. This list is **closed** — adding a variable requires updating ARCHITECTURE.md §8.3 first (AGENTS.md §5). **Version:** 1.5 (2026-11-21). Phase C adds `NETWATCH_EBPF_OBJECT` for the collector object path; Phase A added no environment variables. 1.4: the object is embedded in `netwatchd` from a prebuilt file, so the variable is now a dev-only override with no production default path. 1.5 (2026-11-21): add debug-only `NETWATCH_LICENSE_PUBLIC_KEY` for owner license testing; ignored by release builds.

| Var | Consumer | Default | Purpose |
|---|---|---|---|
| `NETWATCH_DATA_DIR` | netwatchd | `/var/lib/netwatch-pro` | DB + state directory (systemd sets it) |
| `NETWATCH_LOG_DIR` | netwatchd | `<data>/log` | Log output |
| `NETWATCH_LOG_LEVEL` | netwatchd | `info` | `error/warn/info/debug/trace` |
| `NETWATCH_API_SOCKET` | netwatchd + UI | `/run/netwatch-pro/api.sock` | Unix domain socket path; peer-credential UID allowlist authorization |
| `NETWATCH_API_GID` | netwatchd | `SUDO_GID` if set, else root GID | Group id allowed to connect; socket is created `0660` owned by this group |
| `NETWATCH_ALLOWED_UIDS_FILE` | netwatchd | `/etc/netwatch-pro/allowed-uids` | One UID per line (`#` comments). Peer-credential allowlist read on every connection; root, the daemon's own UID and `SUDO_UID` are always allowed. Missing file = no extra UIDs (fail closed) |
| `NETWATCH_RETENTION_DAYS` | netwatchd | `90` | Default retention; user setting overrides (Pro > 90) |
| `NETWATCH_DISABLE_EBPF` | netwatchd | unset | Diagnostic switch: run with empty collector (degraded chip) |
| `NETWATCH_EBPF_OBJECT` | netwatchd (dev only) | unset | Dev override to read the compiled object from a file instead of using the bytes embedded by `build.rs` (built by `cargo run -p xtask -- bpf`); a bad override -> degraded state, not a crash |
| `NETWATCH_DEV_DAEMON_ADDR` | UI (dev only) | unset | Point UI at a dev daemon socket; never read in packaged builds |
| `NETWATCH_LICENSE_PUBLIC_KEY` | netwatch-api (debug builds only) | unset | 64-char Ed25519 public-key hex; dev verifier override for owner testing, ignored in release builds |

Removed: `NETWATCH_API_ADDR`, `NETWATCH_API_TOKEN_FILE` (Unix socket + peer-credential replaces loopback TCP + bearer token).

Secrets: none at rest — the per-boot API token is retired; authorization is via the root-owned allowed-UID list (not a secret value).

Dev template: `.env.example` (repo root) mirrors this list for local runs; copy it to `.env` only if you need to override defaults. Nothing reads `.env` yet — the daemon lands in Phase 1 (PLAN.md P1.1).
