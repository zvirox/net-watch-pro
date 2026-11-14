# AUTH_SECURITY.md — Authentication & Security Design

**Version:** 1.0 · **Date:** 2026-11-14  
**Sources:** `PRD.md` §4/§10 (non-goals, risks) · `ARCHITECTURE.md` §5.3/§7/§8.3 (API, errors, env) · `API.md` §1 (auth, roles, limits) · `DATABASE.md` §6 (access rules) · `REAPM.md` §12 (reference security model)

> **Scope decision:** this is a **local-only desktop app** (PRD §4: no cloud, no accounts, no telemetry). There is no server, therefore no login/signup/password-reset surface exists. This file documents the *actual* security model and marks every classic web-auth item **N/A with a one-line reason** — inventing accounts to satisfy a template would violate PRD non-goals and add attack surface, not security.

---

## 1. Threat Model (what we're actually protecting)

| Asset | Threat | Primary defense |
|---|---|---|
| Root daemon | Local privilege-escalation via its API/files | Loopback-only bind, bearer token, hardened systemd unit, least-privilege capabilities |
| SQLite history (privacy data) | Other local users reading it | Root-owned `0750` dir / `0640` file; UI never opens the DB (DATABASE.md §6) |
| Per-boot API token | Another local process impersonating the UI | `0600` root-owned file; UI reads it via a one-shot privileged helper |
| License keys | Forged Pro entitlements | Offline signature verification, embedded public key; raw keys never stored |
| User trust | The app phoning home | Zero-outbound by design; verifiable in the app's own Live view |

Out of scope (no surface): remote attackers, credential stuffing, session fixation over the internet — there is no listener beyond `127.0.0.1` and no accounts.

---

## 2. Authentication Methods

| Method | Applicable? | One-line reason |
|---|---|---|
| Email + password | **N/A** | No user accounts exist — the local OS user *is* the identity (PRD non-goal: no cloud/accounts). |
| OAuth (Google/GitHub…) | **N/A** | OAuth requires a server-side redirect endpoint and internet identity providers; both violate local-only + zero-outbound. |
| Magic link | **N/A** | Requires email delivery infrastructure — same violation. |
| **Per-boot bearer token** | **Yes — the auth method** | A random secret shared only between daemon and local UI; proves "local trusted process" with zero infrastructure. |
| Offline license key | **Yes — entitlement, not identity** | Proves paid tier via signature, not who the user is (PRD §6.1 #9). |

### 2.1 "Login" equivalent — UI ↔ daemon link handshake

No login screen exists; the equivalent is the daemon link, per ARCHITECTURE.md §5.3:

1. Daemon boots (systemd) → generates a **256-bit random token** → writes `$NETWATCH_API_TOKEN_FILE` with mode `0600`, owner root.
2. UI process launches (user context) → Tauri shell's one-shot privileged helper reads the token file (the *only* privileged read the UI ever triggers) → token held in process memory only.
3. UI presents `Authorization: Bearer <token>` on every HTTP/WS call (API.md §1.1).
4. Failure at any step = `AUTH_INVALID` (401) → honest "Stopped/Connecting" UI state (DESIGN.md §7.5), never a silent retry storm.

### 2.2 Signup — N/A (no accounts). Logout — UI-only action

- "Logout" = quitting the UI. The token lives in UI process memory and dies with the process; **no server-side session to revoke** (one-line reason: stateless local auth has nothing to log out *of*).
- "Password reset" — N/A (no passwords). The equivalent recovery is the daemon itself: token is **regenerated on every daemon restart**, so a lost/leaked token self-heals at next boot.

### 2.3 License activation (the only credential-like flow)

- User pastes a signed key in the Upgrade modal (DESIGN.md §8.8) → daemon verifies signature offline (embedded public key) → stores only a SHA-256 **fingerprint**, `tier`, timestamps (DATABASE.md §2.7).
- Raw license keys are **never persisted** — one-line reason: a fingerprint proves the row corresponds to the verified key without becoming a replayable credential.
- Deactivation (`DELETE /license`) resets to `free`; started export jobs get a grace window (FEATURES.md F9).

---

## 3. Token Design

**JWT: N/A — deliberately not used.** One-line reason: a JWT exists to carry identity claims across untrusted boundaries and servers; here there is one caller (the UI), one verifier (the daemon), and no cross-process trust transfer — an opaque random secret does the same job with less machinery and nothing to leak in claims.

| Property | Design | One-line reason |
|---|---|---|
| Type | Opaque 256-bit random token (CSPRNG) | Unpredictable, no structure to exploit, trivial to verify by constant-time compare. |
| Access token | The per-boot token — authorizes all `local-ui` calls | Single credential is sufficient at this trust boundary. |
| Refresh token | **N/A** | Token is re-readable from the root-owned file via the helper; refresh machinery would add state with no benefit. |
| Expiry | Lifetime = daemon uptime (per-boot) | A daemon restart is a natural, honest "session end" — matching the reference app's per-boot model (REAPM.md §12). |
| Storage (UI) | Process memory only; never written to disk, never in localStorage/cookies | Compromise dies with the process; nothing to steal at rest. |
| Storage (daemon) | `0600`, root-owned token file | Root-only readability is the whole trust boundary (DATABASE.md §6). |
| Rotation | On every daemon restart; **no revocation list** (nothing to revoke) | Short lifetime makes revocation lists pure overhead. |
| Leaked token? | Restart the daemon — old token invalid | Recovery is a documented one-step user action (Diagnostics). |
| Comparison | Constant-time compare on the daemon side | Prevents timing oracles on the local socket. |

---

## 4. Roles & Permissions Matrix

Two runtime roles (API.md §1.1) + one entitlement dimension. The UI's *screens* follow DESIGN.md; the *enforcement* is server-side (PRD R7: gating is never UI-only).

| Capability | anonymous (no token) | local-ui (token) | local-ui + Pro entitlement |
|---|---|---|---|
| `GET /health` | ✅ | ✅ | ✅ |
| All other REST endpoints + WS | ❌ 401 | ✅ | ✅ |
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
- API token: generated, written `0600`, held in memory — never logged, never embedded in URLs (header only).
- No secrets in code, tests, or commits (AGENTS.md §5).

---

## 6. Rate Limiting, CORS, CSRF, XSS

| Control | Applicable? | Design (one-line reason) |
|---|---|---|
| **Rate limiting** | Yes | API.md §1.5: 120 req/min reads, 5/min export creation, 10/min writes/downloads → protects the DB and live path from runaway local clients; 429 + `Retry-After`. |
| **CORS** | Yes (defensively closed) | API serves loopback clients only and sets **no `Access-Control-Allow-Origin`** headers — browsers can't reach it cross-origin, so CORS is deny-by-default by omission. |
| **CSRF** | N/A for HTTP, guarded on WS | No cookie auth exists (pure `Authorization` header), which is the CSRF kill; WS upgrades additionally verify the `Origin` header is absent/loopback-equivalent — a malicious web page cannot open the socket. |
| **XSS** | Yes | All UI content is local + Svelte-escaped; **no remote HTML/Markdown is ever rendered** (contrast: the reference app renders vendor news/markdown — REAPM.md §9 — we render none); strict Tauri CSP (`default-src 'self'`, no `unsafe-inline` scripts), no `eval`, remote-content Tauri capability disabled. |
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

| Rule | One-line reason |
|---|---|
| The **only secret** in this product is the per-boot API token — auto-generated, never user-set (ARCHITECTURE.md §8.3) | A generated secret can't be weak, reused, or committed by a user. |
| **No secrets in code, tests, or git history** — ever (AGENTS.md §5) | Git history is forever; `git filter-repo` is not a plan. |
| License **private signing key** exists only in the release/build infrastructure, never in the repo or shipped binaries | Possession of the private key = ability to mint Pro. |
| Shipped verifier holds the **public key** only (`packaging/licenses/`) | Asymmetric design: public key can verify, never mint. |
| `.env` files: dev-only (`NETWATCH_DEV_DAEMON_ADDR`), `.gitignore`d, never in packaged builds | Dev convenience must never become a shipped config path. |
| Env vars limited to the closed §8.3 list; anything else must be added to ARCHITECTURE.md first | A closed surface is an auditable surface. |
| Token file mode asserted `0600` root at daemon start; refuse to boot if world-readable | A permissive token file silently undoes the entire auth model. |
| Logs: no token values, no license keys, no file contents — only IDs and codes | Logs outlive processes and get pasted into bug reports. |

---

## 9. Daemon Hardening (beyond the API)

- **systemd sandboxing** (PRD §5 contract): `ProtectSystem`, `PrivateTmp`, `ProtectHome=read-only`, `RestrictAddressFamilies`, `MemoryDenyWriteExecute`, `LockPersonality` — the root process is fenced even if a bug exists (mirrors REAPM.md §12).
- **Capability bounding:** only `CAP_BPF`, `CAP_PERFMON`, `CAP_NET_ADMIN` — no `SYS_PTRACE`, no `DAC_OVERRIDE` beyond what eBPF needs.
- **`#![deny(unsafe_code)]`** everywhere except the eBPF crate/loader (ARCHITECTURE.md §8.2) — small audited unsafe surface.
- **No `unwrap()` in runtime paths**; worker panics → degraded state + `daemon_events`, never daemon death (ARCHITECTURE.md §7.1).
- **Fail-fast startup codes** per failure class — systemd and Diagnostics act on them (§7.1).
- **UI is not the security boundary:** stopping/compromising the UI cannot alter data or rules — it can only *view* via the API (REAPM.md §12's proven separation).

---

## 10. OWASP Mapping (desktop-local adaptation)

| OWASP concern | Our answer |
|---|---|
| A01 Broken Access Control | Token-gated API + server-side Pro gating + DB root-only (§4, DATABASE.md §6). |
| A02 Cryptographic Failures | CSPRNG token, constant-time compare, EdDSA/Ed25519-signed licenses, no secrets at rest beyond the root-owned token file. |
| A03 Injection | sqlx bound params + enum-resolved identifiers + whitelist validation (§7). |
| A04 Insecure Design | Threat model first (§1); monitor-only MVP keeps the attack surface minimal; firewall phase-in gets its own security review (PRD R2). |
| A05 Security Misconfiguration | Hardened unit asserted via `systemd-analyze security`; token file mode asserted at boot; closed env-var list. |
| A06 Vulnerable Components | `cargo deny` + `npm audit` in CI (P0.7); pinned toolchains. |
| A07 Auth Failures | No passwords/sessions by design; token per-boot, constant-time, rate-limited (§3, §6). |
| A08 Integrity Failures | Signed packages/repos (P8.8); signed updates channel = package manager itself. |
| A09 Logging Failures | `daemon_events` for every failure class with codes, no secrets in logs (§8). |
| A10 SSRF | N/A — the app makes no HTTP requests; no URL fetchers exist to abuse. |

---

## 11. Pre-Launch Security Checklist

Gate: blocks `v1.0.0` (maps to PLAN P10.5 and PRD DoD #3).

- [ ] `systemd-analyze security netwatchd` reviewed; no unexplained black marks
- [ ] Capability set is exactly `CAP_BPF, CAP_PERFMON, CAP_NET_ADMIN` (nothing more)
- [ ] Token file `0600` root; world-readable → daemon **refuses to boot**
- [ ] API binds `127.0.0.1` only (verified with `ss -tlnp`, not config reading)
- [ ] No-token request → 401 on every endpoint except `/health` (full matrix test)
- [ ] Free-tier token: advanced filters, exports, retention>90 → 403 (raw curl matrix, not UI)
- [ ] WS upgrade rejected without valid token; Origin check active
- [ ] Constant-time token comparison verified (code review + test)
- [ ] sqlx: zero string-interpolated SQL (grep gate in CI)
- [ ] LIKE/path/JSON validation tests for every user-input surface
- [ ] Rate limits fire correctly (429 bodies match API.md §1.3 format)
- [ ] `cargo deny` + `npm audit` clean or waivers documented
- [ ] No secrets in repo history (secret-scan in CI); private signing key confirmed absent
- [ ] Logs redacted: no token/license/raw payload dumps
- [ ] CSP strict; no remote content rendering; Tauri remote-content capability off
- [ ] Zero-outbound claim re-verified on the final build: app's own Live view shows only loopback sockets
- [ ] Threat-model doc (§1) re-reviewed against final architecture; changes logged
- [ ] External security review completed (PRD R2 mitigation) before GA

---

*End of AUTH_SECURITY.md. Changes to auth/secret handling must update this file, ARCHITECTURE.md §8.3, and API.md §1 in the same change (AGENTS.md §4.5).*
