# Security Policy

Net Watch Pro is a **local-only** application: it makes no outbound network
connections, has no accounts, and stores all data on-device (PRD §1/§4).

## Supported Versions

Pre-1.0 security fixes land on `main` and ship in the next release tag.

| Version | Supported |
| --- | --- |
| `main` (all `v0.x` pre-releases) | ✅ |
| older tagged releases | ❌ — upgrade |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security reports.**

1. Preferred: [GitHub Private Vulnerability Reporting](https://github.com/zvirox/net-watch-pro/security/advisories/new) on this repository.
2. Alternative: email **security@netwatchpro.local** *(placeholder — replace with the maintained contact address before first public release; tracked in README §License placeholder list)*.

Include: affected component (`netwatchd` daemon / UI shell / packaging /
installer), version or commit, a minimal reproduction, and your assessment of
impact. You will receive an acknowledgement within **72 hours** and a fix
timeline within **7 days**.

We credit reporters by default (opt out in your report). We ask for
**90 days** coordinated disclosure before public detail; we will not take
legal action against good-faith research that respects user privacy.

## Scope — in

- The `netwatchd` root daemon: privilege boundary, capability use, the
  localhost API (`AUTH_SECURITY.md`), token handling, SQLite data handling.
- The Tauri UI shell: IPC surface, CSP, capability grants, tray.
- Packaging/installer: systemd unit hardening, post-install scripts,
  repo/package signing (INTEGRATIONS.md §3).
- Anything that could make the app break its **zero-outbound** promise.

## Scope — out

- Vulnerabilities in third-party dependencies (report upstream; we track via
  `cargo deny` / `npm audit` in CI).
- Social engineering of users, physical access attacks on unlocked machines.
- The reference analysis in `REAPM.md` (documents a *different* product).
- Reports from automated scanners without a demonstrated impact — we still
  read them, but they are triaged last.

## Hard Design Rules Reporters Should Know

- The UI is **not** a security boundary; it never opens SQLite, never reads
  root-owned files, and all data flows through the token-authenticated
  loopback API (`DATABASE.md` §6, `ARCHITECTURE.md` §5).
- The only secret is the per-boot API token (`0600`, root-owned); license
  keys are verified offline and only stored as fingerprints
  (`AUTH_SECURITY.md` §2.3/§8).
- The daemon runs under a hardened systemd unit with bounded capabilities
  (`CAP_BPF`, `CAP_PERFMON`, `CAP_NET_ADMIN`) — see `AUTH_SECURITY.md` §9.

## Verification

CI runs the security gates from `AUTH_SECURITY.md` §11 that are automatable
(dependency advisories, lint-deny for `unwrap` in daemon paths, secret
scanning). Manual gates are re-run before every release (PLAN.md P10.5).
