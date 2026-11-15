# INTEGRATIONS.md — Third-Party Service Registry

**Version:** 1.1 · **Date:** 2026-11-15  
**Sources:** `PRD.md` §4 (non-goals), §6.1 #9 (license) · `ARCHITECTURE.md` §5.2 ("third-party services: none at runtime"), §8.3 (env vars) · `AUTH_SECURITY.md` §2.3 (license flow) · `docs/decisions.md` (owner decisions D1–D7)

**Version history:** 1.1 (2026-11-15): §4 Future Integration Contract marked non-binding/archived per finding F-10; §4.2 Payments pre-written spec is reference only, not MVP path.

> **⚠ Source-of-truth note (AGENTS.md §3):** the PRD's core promise is **local-only, zero-outbound**. Payments, AI, file storage, and email are therefore **not integrated** — this file records each requested category as N/A with a reason, documents the two real seams that do exist (license purchase handoff, package repos), and defines the service-layer contract any *future* integration must follow. **Adding any N/A category is a PRD change requiring user approval first** — never a code-first decision.

---

## 1. Integration Inventory

| Category (requested) | Status | One-line reason |
|---|---|---|
| **Payments (Stripe / Razorpay)** | **N/A in-app** | Licenses are offline-signed keys; purchase happens out-of-band ("Buy a license" opens an external URL — PRD Flow 4); the app never talks to a payment provider. |
| **AI (OpenAI / Anthropic)** | **N/A** | No AI features exist in MVP/V2/Later (PRD §6); calling an AI API would violate zero-outbound. |
| **File storage (S3 / Supabase)** | **N/A** | All data is local SQLite (PRD §4); exports are written by the user via the native save dialog. |
| **Email** | **N/A** | No accounts → no transactional email; the reference app's pattern is deliberately not copied. |
| **Analytics / telemetry** | **N/A** | Explicitly a product promise (PRD §1); verifiable via the app's own Live view. |
| **Update check service** | **Deferred — PRD OQ3/OQ4 decision** | Updates flow through package managers (F10); any in-app update-metadata check needs a PRD decision first. |
| **License purchase handoff** | **Active (out-of-app)** | See §2 — the one real customer-facing seam. |
| **Package repos (deb / rpm / AUR)** | **Active (build-time)** | See §3 — distribution is the "hosting" (ARCHITECTURE.md §2). |

---

## 2. Integration 1 — License Purchase Handoff (out-of-app)

**Why we use it this way:** the freemium model (PRD §6.1 #9) needs a way to sell Pro, but pulling payment into the app would break the zero-outbound promise and put a card-handling surface inside a root-adjacent product. The chosen split: the **vendor's website/store** runs payments (any provider — Stripe, Razorpay, Lemon Squeezy — swappable without touching the app), and the customer receives an **offline-signed license key** to paste into the app.

| Aspect | Detail |
|---|---|
| **App-side env variables** | **None.** The app never contacts the store. `BUY_LICENSE_URL` is a *build-time constant* in UI config (not a secret; no dynamic override). |
| **SDK/package** | **None in-app.** Keys are produced vendor-side and signed with the Ed25519 private key held only in release infrastructure (AUTH_SECURITY.md §8). |
| **Setup steps** | 1) Choose store/provider (business decision, outside repo) · 2) Vendor store fulfills via a key-generation script that signs licenses with the release private key · 3) Public key embedded in `packaging/licenses/` at build · 4) UI "Buy a license" button opens the store URL in the system browser (user-triggered; app itself makes no request) · 5) User pastes key → `POST /license/activate` verifies offline (API.md §2.8). |
| **Wrapper files (never frontend-direct)** | UI renders only the stored URL constant; all *verification* logic lives in `crates/netwatch-core/src/license.rs` behind the API. The frontend never parses keys or touches the public key path. |
| **Webhooks** | **None in-app.** Key delivery is store-side email/download. If the vendor later wants in-app webhooks (e.g. auto-provisioning), see §4's contract — requires PRD change. |
| **Retries / idempotency / errors** | App side: activation is idempotent by design (409 `CONFLICT` on double-activate, API.md §2.8); invalid/expired/tampered keys → 403 `LICENSE_INVALID` with distinct messages; no network retries exist because no network call exists. |
| **Cost / rate limits** | Store fees are vendor-side (typical: ~3% + fixed per transaction); **app-side cost: zero**; no API rate limits apply. |
| **Fallback if "down"** | The store being down only blocks *new purchases* — activated licenses keep working forever (offline verification, no phoning home); manual key issuance via support is the documented fallback. |
| **Sandbox / test mode** | Test the *app* side with vendor-provided test keys (bad signature, expired, valid): the activation matrix in FEATURES.md F9 acceptance criteria; store-side testing uses the provider's own test mode — invisible to this repo. |

---

## 3. Integration 2 — Package Repositories (build-time)

**Why:** distribution *is* the hosting (ARCHITECTURE.md §2); PRD §6.1 #10 mandates AUR + apt repo + dnf repo with signed updates. Users' machines talk to repos via their package manager — the app itself still makes zero outbound calls.

| Aspect | Detail |
|---|---|
| **Env variables (CI/release only)** | `DEB_SIGNING_KEY` (GPG private key id/passphrase via CI secrets), `RPM_SIGNING_KEY`, `APT_REPO_HOST`, `RPM_REPO_HOST`, `AUR_SSH_KEY` — all placeholders resolved in CI secrets; **never in the repo** (AUTH_SECURITY.md §8). |
| **SDK / tooling** | No SDK — standard packaging tools: `dpkg-deb`/`aptly` (or `reprepro`), `rpmbuild`/`createrepo_c`, `makepkg` (PKGBUILD), wired by `xtask` + the release workflow (PLAN P8.2–P8.8). |
| **Setup steps** | 1) `xtask` builds per-distro artifacts · 2) CI signs .deb (debsigs) and .rpm (rpmsign) · 3) Publish to repo hosts + `createrepo_c`/aptly metadata refresh · 4) AUR: push PKGBUILD update · 5) Verify signatures on clean VMs. |
| **Wrapper files** | `packaging/deb/`, `packaging/rpm/`, `packaging/arch/PKGBUILD`, `.github/workflows/release.yml`, `xtask` — all release infra; no product code depends on repo reachability. |
| **Webhooks / retries / idempotency** | N/A (static file distribution). Repo metadata refresh is idempotent; failed publish → re-run workflow (artifacts are immutable once tagged). |
| **Errors** | Publish failure blocks the release tag (CI gate), never ships unsigned; signature verification failure on user side = package-manager-level block (documented in F10). |
| **Cost / rate limits** | Repo hosting: a small VPS/object storage, dollars/month; no API rate limits. |
| **Fallback if down** | GitHub Releases always carries the same signed artifacts (PLAN §1 packaging matrix); AUR builds from source need no repo. |
| **Sandbox / test** | CI dry-run builds + clean-VM install matrix (P10.4); signature verification tested with the CI public key before rotating to production keys. |

---

## 4. Future Integration Contract (gated — requires PRD change + user approval)

> ⚠ **Non-binding / archived guidance.** Per plan-fix finding F-10, §4.2 (pre-written Payments spec) and §4.1 are **not part of the MVP path**. They are retained only as a reference contract should the owner ever decide to add a third-party API. Any addition requires a PRD change + user approval (AGENTS.md §3) — never a code-first decision.

If a future version legitimately needs a third-party API (e.g. an opt-in update-check channel resolving PRD OQ3, or in-app license provisioning), it must be built exactly this way — decided **now** so a swap later is cheap:

### 4.1 The service-layer pattern (binding)

1. **One wrapper per provider, daemon-side only.** A small crate module (e.g. `crates/netwatch-core/src/payments.rs`) exposing a narrow trait (`trait PaymentsProvider`), with a thin adapter per vendor. One-line reason: the vendor's types and SDK never leak past the adapter, so swapping Stripe → Razorpay touches one file.
2. **The frontend never calls a third-party service directly** (hard rule, mirrors ARCHITECTURE.md §5/API.md): UI → daemon API → wrapper. One-line reason: one trust boundary, one place for auth/retries/logging, no secrets in the webview.
3. **Capabilities are config-driven:** provider selection + credentials come from the closed env-var list (new vars must be added to ARCHITECTURE.md §8.3 first — e.g. `NETWATCH_PAYMENTS_PROVIDER`, `NETWATCH_PAYMENTS_SECRET_KEY_FILE` pointing at a `0600` file, never inline).
4. **Zero-outbound stays opt-in:** any new outbound call is off by default, disclosed in Settings/About, and re-verifiable in the app's own Live view (PRD trust metric).
5. **Feature flag + fallback:** every wrapper has a no-op/offline implementation used when the env is unset — the app must run fully without any provider (proven by CI's "no env" build).

### 4.2 Pre-written spec: Payments (only plausible future case)

| Aspect | Contract |
|---|---|
| **Why (if ever)** | In-app upgrades/trials for Pro without sending users to a browser — a *conversion* optimization, never a requirement. |
| **Env vars** | `NETWATCH_PAYMENTS_PROVIDER=stripe|razorpay|none`, `NETWATCH_PAYMENTS_SECRET_KEY_FILE` (`0600` key file), `NETWATCH_PAYMENTS_WEBHOOK_SECRET_FILE`. Placeholders only in docs (AUTH_SECURITY.md §8). |
| **SDK** | Vendor's official server SDK as a **Rust crate behind the adapter** (`stripe-rust` / `razorpay` HTTP client) — added only after user approves the dependency (AGENTS.md §5). |
| **Setup steps** | 1) PRD amendment + user go-ahead · 2) Adapter crate + trait · 3) Checkout-session endpoint in API v1 (additive per API.md §1.6) · 4) Webhook receiver path (`/api/v1/webhooks/payments`, token-exempt like `/health`, signature-verified) · 5) Key provisioning: purchase → daemon requests a signed license from the vendor fulfillment service → activate flow unchanged. |
| **Webhook handling** | Verify signature first (constant-time compare); process only event types on an allowlist; **idempotency by event ID** — a `processed_events` table (new migration + DATABASE.md update first) keyed by provider event id, `ON CONFLICT DO NOTHING`; unknown/duplicate events ack'd without action. |
| **Retries** | Outbound: capped exponential backoff (1 s → 16 s, 5 tries) via Tokio; webhook side: provider retries are safe because of event-id idempotency — the receiver always returns 2xx after processing or permanent failure, never 5xx for handled duplicates. |
| **Error handling** | Every provider error maps into the §1.3 error envelope with a stable code (e.g. `PAYMENTS_UNAVAILABLE` added to ARCHITECTURE.md §7.2 first); user-facing copy never exposes vendor internals; a payment failure never corrupts license state (transactional, verify-then-store fingerprint). |
| **Cost / rate limits** | Stripe/Razorpay: ~2–3% + fixed per transaction; API rate limits are generous (checkout sessions ~100/s) — our volume is desktop-scale; still enforce client-side caps (reuse API.md §1.5 export-tier discipline). |
| **Fallback if down** | Offline license keys remain the permanent fallback path (§2) — checkout down degrades to "Buy a license" external flow automatically (wrapper reports unavailable, UI links out). |
| **Sandbox / test mode** | Provider test mode keys via env (test cards from vendor docs); webhook receiver tested with the provider CLI `stripe listen --forward-to …`; the §5 connection test asserts: sandbox purchase → webhook → license activated → fingerprints match, and duplicate webhook delivery is a no-op. |

### 4.3 Categories that will likely never be integrated

- **AI APIs:** no product feature justifies the outbound call; revisit only if a V3+ feature spec demands it (PRD change).
- **File storage / email / analytics:** local-only architecture removes the need permanently; the export path (user's own disk) and notifications (system-level) already cover those jobs.

---

## 5. Testing & Verification Summary

| Layer | How to verify |
|---|---|
| App zero-outbound claim | Final-build test: the app's own Live view shows only loopback sockets (AUTH_SECURITY.md §11; FEATURES.md F10). |
| License activation matrix | Vendor test keys (valid / expired / tampered / malformed) → expected 200/403/409 per API.md §2.8 — no network involved. |
| Package repos | CI dry-run + clean-VM install on all 3 distro families (PLAN P10.4); signature verify before key rotation. |
| Any future provider | §4.1 pattern + §4.2 webhook/idempotency test; **plus** a CI job that builds with provider env unset and asserts full app functionality (fallback path is the default). |

---

## 6. Governance

- Adding, swapping, or removing any integration = update this file + (if it affects the product's promises) PRD/ARCHITECTURE.md, with user approval — never code-first.
- The swap-later rule (§4.1) is binding for *any* vendor-specific code: if a file name contains a vendor name, it must contain only an adapter implementing a neutral trait.

---

*End of INTEGRATIONS.md. Today's truth: the product's integration surface is the license handoff and package repos — everything else is a documented, gated contract, not code.*
