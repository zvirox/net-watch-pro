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

The locked release packaging matrix runs in clean CI **containers** (`ubuntu:22.04`, `ubuntu:24.04`, `fedora:latest`, `archlinux:latest` via `zvirox/release-workflows`). Containers share the host kernel and verify package installation, desktop entries, and removal cleanly; separate manual VM release-package test requirements are retired.

## Future privileged components

Per `docs/ROADMAP.md` §3 and `docs/decisions.md` D13, any future code that executes
with root or `CAP_NET_ADMIN` privileges requires an explicit security review entry
in this document before implementation begins. This includes:

- **DNS packet observation (v1.2):** Passive capture of local DNS queries and responses
  (UDP/TCP 53 or resolved monitor socket). Security evaluation must cover parser memory
  safety, domain spoofing defenses, and preventing DNS leak vulnerabilities.
- **Active network probes (v1.3):** ICMP echo or UDP latency/jitter/loss probes. Security
  evaluation must verify restriction to unprivileged ping sockets (`IPPROTO_ICMP` via
  `net.ipv4.ping_group_range`), strict rate limiting, and zero exposure as a local
  amplification or scanning vector.
- **Traffic control & blocking (`netwatch-guard`, v2.0):** If approved, application
  network blocking will run as an isolated standalone daemon separate from `netwatchd`.
  Security evaluation must cover dedicated polkit authorization rules, nftables ruleset
  isolation, and fail-open guarantees ensuring network access cannot be severed by
  monitoring failures.
- **Plugins and extensions (v2.0):** No third-party or user-supplied code may run within
  the root daemon context. Security review must validate that extensions execute
  unprivileged in user context against the local Unix socket API.

