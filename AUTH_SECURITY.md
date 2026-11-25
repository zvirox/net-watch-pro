# AUTH_SECURITY.md — Authentication & Security Design

**Version:** 1.3.2 · **Date:** 2026-11-21
**Sources:** `PRD.md` §4/§10 (non-goals, risks) · `ARCHITECTURE.md` §5.3/§7/§8.3 (API, errors, env) · `API.md` §1 (auth, roles, limits) · `DATABASE.md` §6 (access rules) · `REAPM.md` §12 (reference security model) · `docs/decisions.md` (owner decisions D1a–D5a)

**Version history:** 1.3.1 (2026-11-17): define the offline license key wire format and embedded public-key location. 1.3 (2026-11-15): peer-credential UID allowlist replaces per-boot token + Polkit helper (F-06/D1a); CORS/CSRF/Origin become N/A on Unix socket; capability list updated for measured set (F-05/D5a); secrets section reflects no per-boot token at rest. 1.3.2 (2026-11-21): debug-only `NETWATCH_LICENSE_PUBLIC_KEY` override for owner testing of Pro/free flows; compiled out of release binaries, so shipped verifiers still trust only the embedded public key. 

> **Scope decision:** this is a **local-only desktop app** (PRD §4: no cloud, no accounts, no telemetry). There is no server, therefore no login/signup/password-reset surface exists. This file documents the *actual* security model and marks every classic web-auth item **N/A with a one-line reason** — inventing accounts to satisfy a template would violate PRD non-goals and add attack surface, not security.

---

## 1. Threat Model (what we're actually protecting)

| Asset | Threat | Primary defense |
|---|---|---|
| Root daemon | Local privilege-escalation via its API/files | Unix socket only (no loopback TCP), peer-credential UID allowlist, hardened systemd unit, least-privilege capabilities (D1a) |
| SQLite history (privacy data) | Other local users reading it | Root-owned `0750` dir / `0640` file; UI never opens the DB (DATABASE.md §6) |
| Allowed-UID list | Another local user reading all-users traffic data | Root-owned socket dir `0750` / socket `0660`; peer-credential (`SO_PEERCRED`) check vs allowed-UID list; one admin prompt at onboarding; on multi-user machines only allowed UIDs can read traffic data |
| License keys | Forged Pro entitlements | Offline signature verification, embedded public key; raw keys never stored |
| User trust | The app phoning home | Zero-outbound by design; verifiable in the app's own Live view |

Out of scope (no surface): remote attackers, credential stuffing, session fixation over the internet — there is no listener beyond the Unix socket and no accounts.

---

## 2. Authentication Methods

| Method | Applicable? | One-line reason |
|---|---|---|
| Email + password | **N/A** | No user accounts exist — the local OS user *is* the identity (PRD non-goal: no cloud/accounts). |
| OAuth (Google/GitHub…) | **N/A** | OAuth requires a server-side redirect endpoint and internet identity providers; both violate local-only + zero-outbound. |
| Magic link | **N/A** | Requires email delivery infrastructure — same violation. |
| **Peer-credential UID allowlist** | **Yes — the auth method** | Unix socket `SO_PEERCRED` check vs root-owned allowed-UID list; one admin prompt at onboarding, none per launch (D1a). Replaces the retired per-boot bearer token. |
| Offline license key | **Yes — entitlement, not identity** | Proves paid tier via signature, not who the user is (PRD §6.1 #9). |

### 2.1 "Login" equivalent — UI ↔ daemon link handshake

No login screen exists; the equivalent is the daemon link, per ARCHITECTURE.md §5.3:

1. Daemon boots (systemd) → creates Unix socket `/run/netwatch-pro/api.sock` (mode `0660`, dir `0750`, root-owned).
2. Onboarding (one time, pkexec via dedicated helper) → admin authorizes the policy action `pro.netwatch.enable-monitoring`. The root helper `/usr/libexec/netwatch-pro/enable-monitoring` reads `PKEXEC_UID` from its environment, appends the user's UID to `/etc/netwatch-pro/allowed-uids` (one numeric UID per line, mode `0644`, root-owned), ensures the socket group allows connection, and runs `systemctl daemon-reload && systemctl enable --now netwatchd.service`.
3. UI process launches (user context) → connects to the Unix socket → daemon checks the peer's UID via `SO_PEERCRED` against the allowed-UID list → authorized or refused.
4. UI presents no token on requests — authorization is the socket peer-credential itself (API.md §1.1).
5. Failure at any step (UID not in list, socket missing, permissions) → connection refused → honest "auth-invalid" or "Stopped/Connecting" UI state (DESIGN.md §7.5), never a silent retry storm.

No token file, no per-boot token helper, no `NETWATCH_API_TOKEN_FILE`, no bearer header (D1a).

### 2.2 Polkit action & onboarding helper (`pro.netwatch.enable-monitoring`)

Privilege escalation during onboarding is restricted to a dedicated Polkit action and a single-purpose root helper:

- **Policy ID:** `pro.netwatch.enable-monitoring` (`packaging/polkit/pro.netwatch.enable-monitoring.policy`).
- **Target executable:** `/usr/libexec/netwatch-pro/enable-monitoring` (mode `0755`, owned by `root:root`).
- **Prompt text:** *"Allow Net Watch Pro to monitor network traffic"*.
- **Authorization scope:** The prompt authorizes strictly:
  1. Reading the caller's invoking UID from `PKEXEC_UID` (numeric integer `1 <= uid <= 4294967294`).
  2. Appending this UID to `/etc/netwatch-pro/allowed-uids` idempotently (mode `0644`, atomic write via temp file + rename).
  3. Configuring `/etc/systemd/system/netwatchd.service.d/10-socket-group.conf` and user group membership.
  4. Executing `systemctl daemon-reload` and `systemctl enable --now netwatchd.service`.
- **Hardening:** The helper accepts **no free-form command-line arguments** (rejects any arguments). The Tauri UI invokes `pkexec /usr/libexec/netwatch-pro/enable-monitoring` directly (no `sh -c`, no shell string interpolation). Exit status is non-zero with a short stderr message on any failure.

### 2.3 Signup — N/A (no accounts). Logout — UI-only action

- "Logout" = quitting the UI. There is no token in memory to clear; the socket authorization is per-connection via `SO_PEERCRED` (stateless local auth).
- "Password reset" — N/A (no passwords, no tokens). Recovery is the daemon itself: the allowed-UID list persists across restarts; a compromised UID is removed by re-running onboarding.

### 2.4 License activation (the only credential-like flow)

- Key format: `NWP1-<payload>-<signature>` with standard unpadded base64 segments. Payload bytes are version `1`, tier `1` (Pro), an 8-byte big-endian expiry (`0` = perpetual), and a 16-byte nonce. The Ed25519 signature covers the payload bytes.
- Verifier uses the public key at `packaging/licenses/nwp-public.hex`; the private signing key stays in release infrastructure and is never committed. Debug builds alone accept a caller-supplied verifier key via `NETWATCH_LICENSE_PUBLIC_KEY` so the owner can mint test keys locally; release builds ignore the variable entirely.
- User pastes a signed key in the Upgrade modal (DESIGN.md §8.8) → daemon verifies signature offline (embedded public key) → stores only a SHA-256 **fingerprint**, `tier`, timestamps (DATABASE.md §2.7).
- Raw license keys are **never persisted** — one-line reason: a fingerprint proves the row corresponds to the verified key without becoming a replayable credential.
- Deactivation (`DELETE /license`) resets to `free`; started export jobs get a grace window (FEATURES.md F9).

---

## 3. Allowed-UID List Design (replaces Token Design — D1a)

**JWT: N/A — deliberately not used.** One-line reason: a JWT exists to carry identity claims across untrusted boundaries and servers; here there is one caller (the UI), one verifier (the daemon), and no cross-process trust transfer — the socket peer-credential does the same job with less machinery.

| Property | Design | One-line reason |
|---|---|---|
| Mechanism | `SO_PEERCRED` on Unix socket; UID checked vs root-owned allowed-UID list | Proves "local trusted process" with zero infrastructure; no token to leak |
| Allowed-UID list | Root-owned file; one UID added during onboarding pkexec; persists across daemon restarts | Single source of who may read traffic data |
| Comparison | Exact UID match (integer comparison, no timing side-channel) | Simple, constant-time by nature |
| Access | Any UID in the list → `local-ui` role; any UID not in list → connection refused | Single credential sufficient at this trust boundary |
| Storage (daemon) | Root-owned file (mode `0600`), dir `/run/netwatch-pro` (mode `0750`) | Root-only readability is the whole trust boundary |
| Rotation | UID removed by re-running onboarding; no auto-expiry | Short-lived sessions not needed; list is static until admin changes it |
| Leaked/compromised UID? | Remove from list via onboarding; restart daemon | Recovery is a documented admin action |
| Multi-user | On multi-user machines, only allowed UIDs can read (all-users) traffic data | Documented in threat model (§1) and PRD Flow 1 |

---

## 4. Roles & Permissions Matrix

Two runtime roles (API.md §1.1) + one entitlement dimension. The UI's *screens* follow DESIGN.md; the *enforcement* is server-side (PRD R7: gating is never UI-only).

| Capability | anonymous (no UID in list) | local-ui (UID in list) | local-ui + Pro entitlement |
|---|---|---|---|
| `GET /health` | ✅ | ✅ | ✅ |
| All other REST endpoints + socket stream | ❌ connection refused | ✅ | ✅ |
| Basic history filters (`app_id`, `app_name`, `open`) | ❌ | ✅ | ✅ |
| Advanced history filters (date range, host, port, protocol combos) | ❌ | ❌ 403 | ✅ |
| `POST /exports` + content | ❌ | ❌ 403 | ✅ (5/min) |
| `PUT /settings` retention > 90 | ❌ | ❌ 403 | ✅ (≤ 365) |
| `PATCH /interfaces/{id}`, `PUT /settings` (other keys) | ❌ | ✅ | ✅ |
| License get/activate/deactivate | ❌ | ✅ | ✅ |
| Database direct access (any table) | ❌ root-only | ❌ **never** — API only (DATABASE.md §6) | ❌ **never** |

| Server capability (daemon-internal) | Who |
|---|---|
| Write `connections`/rollups/`apps` | collector/writer tasks only — no API path writes observed data |
| Run pruner / compaction | scheduled daemon tasks; tier-aware (license row) |
| Start/stop monitoring (pkexec) | user via Tauri shell — outside the API entirely |

---

## 5. Password Hashing

**N/A — no passwords exist.** One-line reason: there is nothing to hash; the secret is server-generated random, never user-chosen. (Bcrypt/argon2 would be dead code here.)

Applied *analog* discipline, so the rule "never store plain-text secrets" still holds:

- License keys: verified then **discarded** — only fingerprint stored (§2.3).
- API token: generated, atomically replaced at `/var/lib/netwatch-pro/api-token` as root-owned `0600`, held in process memory — never logged, never embedded in URLs (header only). Development/test `NETWATCH_API_TOKEN_FILE` override is unavailable to the fixed-path helper.
- No secrets in code, tests, or commits (AGENTS.md §5).

---

## 6. Rate Limiting, CORS, CSRF, XSS

| Control | Applicable? | Design (one-line reason) |
|---|---|---|
| **Rate limiting** | Yes | API.md §1.5: 600 req/min reads, 5/min export creation, 10/min writes/downloads → minimal limits for a single trusted local client; 429 + `Retry-After`. |
| **CORS** | **N/A** | Unix socket, not HTTP — browsers cannot reach it; no `Access-Control-Allow-Origin` headers needed (D1a). |
| **CSRF** | **N/A** | No cookie auth; pure `SO_PEERCRED` on a Unix socket — a malicious web page cannot open the socket (D1a). |
| **XSS** | Yes | All UI content is local + React-escaped; **no remote HTML/Markdown is ever rendered**; strict Tauri CSP (`default-src 'self'`, no `unsafe-inline` scripts), no `eval`, remote-content Tauri capability disabled. |
| Clickjacking | N/A | Tauri window, not a browser frame; no third-party framing surface. |

---

## 7. Input Validation & Sanitization Rules

1. **Whitelist, never blocklist** — every query param validated against its endpoint whitelist (API.md §2); unknown keys → 400 `QUERY_INVALID` naming the field.
2. **SQL injection:** sqlx **bound parameters only**; dynamic bits (sort field, filter keys) resolved against Rust enums, never string-interpolated into SQL.
3. **LIKE escaping:** user-substring searches escape `%`/`_` before use (FEATURES.md F5).
4. **Path traversal:** the icon resource endpoint serves only from the registered icon set — request paths mapped through an ID handle, never passed to the filesystem raw.
5. **License key format:** strict prefix+length check before signature verification (fail fast on garbage).
6. **Bounds everywhere:** pagination `limit ≤ 500`, `top_limit ≤ 10`, retention 7–365 with tier ceiling, `since ≤ until` — server-side, every time (client validation is UX, not security).
7. **JSON bodies:** strict shape parsing; unknown fields rejected on writes (settings/interfaces), ignored on reads (forward compat, API.md §1.6).
8. **Size limits:** request bodies ≤ 16 KB; export filter sets ≤ 1 KB — a local API doesn't need to accept big payloads.
---

## 8. Secrets Management & Environment Variables

## 8. Secrets Management & Environment Variables

| Rule | One-line reason |
|---|---|
| **No per-boot API token** — retired (D1a); authorization is via the root-owned allowed-UID list (§3) | The token was the only secret; it is replaced by a static UID list with no secret value |
| **No secrets in code, tests, or git history** — ever (AGENTS.md §5) | Git history is forever; `git filter-repo` is not a plan. |
| License **private signing key** exists only in the release/build infrastructure, never in the repo or shipped binaries | Possession of the private key = ability to mint Pro. |
| Shipped verifier holds the **public key** only (`packaging/licenses/`) | Asymmetric design: public key can verify, never mint. |
| `.env` files: dev-only (`NETWATCH_DEV_DAEMON_ADDR`), `.gitignore`d, never in packaged builds | Dev convenience must never become a shipped config path. |
| Env vars limited to the closed ARCHITECTURE.md §8.3 list; anything else must be added there first | A closed surface is an auditable surface. `NETWATCH_API_ADDR` and `NETWATCH_API_TOKEN_FILE` removed (D1a); `NETWATCH_API_SOCKET` added. |
| Logs: no license keys, no file contents — only IDs and codes | Logs outlive processes and get pasted into bug reports. |

---

## 9. Daemon Hardening (beyond the API)

- **systemd sandboxing** (PRD §5 contract): `ProtectSystem`, `PrivateTmp`, `ProtectHome=read-only`, `RestrictAddressFamilies`, `MemoryDenyWriteExecute`, `LockPersonality` — the root process is fenced even if a bug exists (mirrors REAPM.md §12).
- **Capability bounding:** only `CAP_BPF`, `CAP_PERFMON`, `CAP_NET_ADMIN` — no `SYS_PTRACE`, no `DAC_OVERRIDE` beyond what eBPF needs. **[HYPOTHESIS]** the measured set from spike Q6 may add `CAP_DAC_READ_SEARCH` if `/proc` fallback proves unavoidable; any addition must be justified here and added to the threat model (F-05/D5a).
- **`#![deny(unsafe_code)]`** everywhere except the eBPF crate/loader (ARCHITECTURE.md §8.2) — small audited unsafe surface.
- **No `unwrap()` in runtime paths**; worker panics → degraded state + `daemon_events`, never daemon death (ARCHITECTURE.md §7.1).
- **Fail-fast startup codes** per failure class — systemd and Diagnostics act on them (§7.1).
- **UI is not the security boundary:** stopping/compromising the UI cannot alter data or rules — it can only *view* via the API (REAPM.md §12's proven separation).

---

## 10. OWASP Mapping (desktop-local adaptation)

| OWASP concern | Our answer |
|---|---|
| A01 Broken Access Control | Peer-credential UID allowlist + server-side Pro gating + DB root-only (§4, DATABASE.md §6). |
| A02 Cryptographic Failures | Offline-signed licenses (EdDSA/Ed25519), no secrets at rest beyond the root-owned allowed-UID list (not a secret value). |
| A03 Injection | sqlx bound params + enum-resolved identifiers + whitelist validation (§7). |
| A04 Insecure Design | Threat model first (§1); monitor-only MVP keeps the attack surface minimal; firewall phase-in gets its own security review (PRD R2). |
| A05 Security Misconfiguration | Hardened unit asserted via `systemd-analyze security`; token file mode asserted at boot; closed env-var list. |
| A06 Vulnerable Components | `cargo deny` + `npm audit` in CI (P0.7); pinned toolchains. |
| A07 Auth Failures | No passwords/sessions by design; peer-credential UID allowlist, rate-limited (§3, §6). |
| A08 Integrity Failures | Signed packages/repos (P8.8); signed updates channel = package manager itself. |
| A09 Logging Failures | `daemon_events` for every failure class with codes, no secrets in logs (§8). |
| A10 SSRF | N/A — the app makes no HTTP requests; no URL fetchers exist to abuse. |

---

## 11. Pre-Launch Security Checklist

Gate: blocks `v1.0.0` (maps to PLAN P10.5 and PRD DoD #3).

- [ ] `systemd-analyze security netwatchd` reviewed; no unexplained black marks
- [ ] Capability set is exactly `CAP_BPF, CAP_PERFMON, CAP_NET_ADMIN` (nothing more)
- [ ] Socket path `/run/netwatch-pro/api.sock` (mode `0660`, dir `0750`, root-owned)
- [ ] Allowed-UID list file mode `0600` root-owned; world-readable → daemon **refuses to boot**
- [ ] Socket accepts only UIDs in allowed-UID list; others refused (connection matrix test)
- [ ] API binds Unix socket only — no loopback TCP listener (verified with `ss -xlp`, not config reading)
- [ ] Free-tier UID: advanced filters, exports, retention>90 → 403 (raw request matrix, not UI)
- [ ] `sqlx`: zero string-interpolated SQL (grep gate in CI)
- [ ] LIKE/path/JSON validation tests for every user-input surface
- [ ] Rate limits fire correctly (429 bodies match API.md §1.3 format)
- [ ] `cargo deny` + `npm audit` clean or waivers documented
- [ ] No secrets in repo history (secret-scan in CI); private signing key confirmed absent
- [ ] Logs redacted: no license/raw payload dumps
- [ ] CSP strict; no remote content rendering; Tauri remote-content capability off
- [ ] Zero-outbound claim re-verified on the final build: app's own Live view shows only loopback sockets
- [ ] Threat-model doc (§1) re-reviewed against final architecture; changes logged
- [ ] External security review completed (PRD R2 mitigation) before GA

---

*End of AUTH_SECURITY.md. Changes to auth/secret handling must update this file, ARCHITECTURE.md §8.3, and API.md §1 in the same change (AGENTS.md §4.5).*
