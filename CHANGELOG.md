# Changelog

All notable changes to Net Watch Pro are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) ·
Versioning: SemVer with pre-release tags `vX.Y.Z-alpha.N` / `-beta.N` (PLAN.md §1).
Categories: Added / Changed / Fixed / Removed / Security.

---

## [v0.0.1-alpha.1] — 2026-11-14

Phase 0 — Foundation & Tooling (PLAN.md §2). Documentation-phase → scaffolded
workspace; no product features yet.

### Added

- **Specification set (source of truth, priority order per AGENTS.md §2):**
  `PRD.md`, `DESIGN.md`, `ARCHITECTURE.md`, `DATABASE.md`, `API.md`, `REAPM.md`,
  plus working docs `AGENTS.md`, `PLAN.md`/`CHECKLIST.md`, `FEATURES.md`,
  `DATABASE.md`, `API.md`, `AUTH_SECURITY.md`, `MCP.md`, `INTEGRATIONS.md`,
  `TESTING.md`, `README.md`.
- **Cargo workspace** (`Cargo.toml`, `rust-toolchain.toml` @ 1.99.1, `deny.toml`,
  `clippy.toml`, `.editorconfig`, `.gitignore`) with five crate skeletons:
  `netwatch-common`, `netwatch-core`, `netwatchd` (binary stub, fail-fast),
  `netwatch-api`, `xtask` (P0.1, P0.2).
- **UI workspace** `app/`: Vite 7 + Svelte 5 (runes-only) + TypeScript strict
  (`noUncheckedIndexedAccess`) + ESLint flat + Prettier (P0.3).
- **Tauri 2 shell** `app/src-tauri/`: least-privilege capability
  (`core:default`), strict CSP, min window 960×600 (DESIGN.md §5.4), placeholder
  brand icons in primary teal; thin `main.rs` with explicit fail-fast startup
  (P0.4).
- **Token CSS layer** `app/src/lib/theme/tokens.css`: full DESIGN.md §3–§6
  transcription — light+dark palettes, chart series, type scale, 4px spacing,
  radius, shadows, motion (with `prefers-reduced-motion` collapse), layout
  variables (P0.6, CSS half).
- **CI** `.github/workflows/ci.yml`: Rust fmt/clippy/test + UI
  check/lint/test jobs with caching (P0.7).
- **Doc mirrors** `docs/api-contract.md`, `docs/db-schema.md`,
  `docs/env-vars.md` bound to `API.md`/`DATABASE.md`/`ARCHITECTURE.md` with a
  drift-fails-CI rule (P0.8).
- **Governance:** dual license (AGPL-3.0-or-later + commercial, `LICENSE`,
  `README.md` §License), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `SECURITY.md`, this `CHANGELOG.md`.

### Security

- Zero-outbound product contract documented and enforced by design from day one
  (`AUTH_SECURITY.md` §1): no accounts, no telemetry, no update pings.
- Token/licence secrets model defined before any code that could leak them.

---

[Unreleased]: https://github.com/zvirox/net-watch-pro/compare/v0.0.1-alpha.1...HEAD
[v0.0.1-alpha.1]: https://github.com/zvirox/net-watch-pro/releases/tag/v0.0.1-alpha.1
