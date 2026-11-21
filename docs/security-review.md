# Security review — Phase 10 P10.5

**Date:** 2026-11-21 · **Scope:** systemd sandbox, Unix-socket peer-credential
authorization, API surface, Tauri shell/CSP, license verifier.
**Method:** source review of the current `main` (`ff2dce3`+), cross-checked
against `AUTH_SECURITY.md`, `ARCHITECTURE.md` and the accepted eBPF spike report.
This is a code review, not a penetration test; items needing a real host are
labelled as such.

## Answer to the standing open question

The UID allowlist **is enforced** in code. `serve_unix` wraps the listener in
`PeerCredListener` (`crates/netwatch-api/src/lib.rs:106-108`); `accept()` calls
`stream.peer_cred()` and applies `PeerPolicy::accept_decision()` before returning
the connection (`crates/netwatch-api/src/peer.rs:180-205`). Disallowed peers are
dropped before HTTP parsing. The old warning in `docs/LIVE-THROUGHPUT-PLAN.md`
has been corrected. The stale placeholder module `netwatch-api/src/auth.rs` was
removed — `peer.rs` is the single authorization authority.

## Findings and disposition

| Severity | Finding | Disposition |
|---|---|---|
| Medium | Stale `auth.rs` placeholder implied auth was not applied | **Fixed**: module deleted; `peer.rs` documented as authority |
| Medium | Accept-loop rejection path untested when suite runs as root | **Fixed**: `accept_decision()` is the exact gating call; unit test rejects a foreign UID regardless of runner UID; listener integration test still covers the socket path for non-root |
| Low–Medium | `CAP_NET_ADMIN` looks broad for a monitor | **Accepted**: spike report Q4 verified it required on this host (`docs/spike-ebpf-report.md:143,624`); contained by `RestrictAddressFamilies=AF_UNIX`, `IPAddressDeny=any`, `ProtectKernelTunables` |
| Low | Allowlist file read without ownership/mode/symlink checks | **Fixed**: `read_allowed_uids()` fails closed on symlink, group/other-writable mode, or non-root ownership when running as root; regression test added |
| Low | Raw sqlx text leaked in 500 responses via `db_error()` | **Accepted for beta**: only authorized local peers can see it; changing messages is a follow-up, not a P1/P2 issue |
| Low | Allowlist re-read on every rejected connect (disk I/O amplification) | **Accepted for beta**: socket is `0660` and local; warning logs are already rate-limited; caching deferred |
| Low | `UMask=0022` on a root daemon | **Fixed**: tightened to `0077` |
| Low | Export job IDs use 2 random bytes; export save path follows symlinks | **Accepted**: no cross-user boundary (all allowed UIDs have full API access); path comes from the OS picker and webview is trusted |

## Verified OK (no change)

- **systemd sandbox:** `NoNewPrivileges`, `RestrictSUIDSGID`, `LockPersonality`,
  `MemoryDenyWriteExecute`, `ProtectSystem=strict`, `ProtectHome/KernelLogs/
  KernelModules/KernelTunables/ControlGroups/Clock/Hostname`, `PrivateTmp`,
  `RemoveIPC`, `RestrictNamespaces`, `RestrictRealtime`, native syscalls,
  `ReadWritePaths` limited to state/runtime dirs. `PrivateNetwork` intentionally
  omitted so the daemon can see host interfaces.
- **Peer credentials:** kernel creds (no TOCTOU), root/self/sudo allow rules,
  fail-closed on missing/malformed file, allowlist re-read after onboarding
  edits. Socket `0660` owned by the configured group; no token fallback.
- **API:** Unix socket only (no TCP), body-size caps, typed IDs, closed setting
  keys, parameterized SQL, LIKE escaping, sort/limit whitelists, export rate
  limits, only `/health` unauthenticated at app layer (still behind socket gate).
- **Tauri/zero-outbound:** capabilities limited to `core:default`, save dialog
  and notifications; CSP has no external origins; no HTTP/fs/shell plugins;
  WS uses `UnixStream`; the only HTTPS anchors are user-triggered docs links.
- **License:** release builds verify only the embedded public key; the debug
  override is documented and compiled out of release. Strict length/prefix
  checks, `verify_strict`, version/tier/expiry validation, fingerprint-only
  storage.

## Residual risk

The distro matrix currently runs in CI **containers** (owner-approved deviation
from PLAN P10.4). Containers cannot validate kernel/BPF variance; the real VM
matrix (Arch/CachyOS, Ubuntu LTS, Fedora) remains pending and blocks DoD #1.
