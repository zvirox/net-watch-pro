# AGENTS.md — Rulebook for AI Coding Assistants

**Every AI assistant working in this repo must follow these rules.** They are binding, not suggestions. When a rule is unclear, ask the user. When a rule conflicts with a source-of-truth file, see §3.

---

## 1. Project Summary (5 lines)

1. **Net Watch Pro** is a Linux network-traffic monitor and per-app data-usage tracker — monitor-only MVP, firewall features come later, step by step (PRD §12).
2. Two processes: a root systemd daemon (`netwatchd`, Rust + Tokio + Aya eBPF) that observes traffic and writes SQLite, and an unprivileged Tauri 2 + Svelte 5 UI. They talk only over a localhost token-authenticated API (ARCHITECTURE.md).
3. Everything is local: no cloud, no telemetry, no outbound connections — this is a product promise and a hard constraint (PRD §1/§4).
4. Freemium: free tier = live monitoring + per-app usage + 90-day history; Pro (offline license key) = advanced filters, CSV/JSON export, longer retention.
5. The docs are the contract: UI follows DESIGN.md, schema follows DATABASE.md, endpoints follow API.md, structure follows ARCHITECTURE.md. Code that drifts from them is wrong.

## 2. Source of Truth — Priority Order

When documents disagree, the **higher** file wins for its domain; ask the user when domains overlap and conflict.

1. **`PRD.md`** — what we build and why (scope, features, non-goals)
2. **`DESIGN.md`** — how the UI looks and behaves (tokens, components, states, a11y)
3. **`ARCHITECTURE.md`** — stack, folder structure, data flow, naming, env vars
4. **`DATABASE.md`** — schema, indexes, migrations, access rules
5. **`API.md`** — endpoints, error format, pagination, versioning
6. **`REAPM.md`** — reference teardown; background/inspiration only, never binding

Also binding (generated/referenced): `docs/api-contract.md`, `docs/db-schema.md`. Treat mismatches between them and API.md/DATABASE.md as bugs — fix the doc or the code, never ignore it.

---

## 3. Conflict Rule — STOP AND ASK

**If a user request conflicts with any source-of-truth file, stop and ask the user before proceeding.** Do not silently pick a side, do not "fix" the doc on your own authority, do not build a one-off exception.

How to ask: name the conflict precisely — request vs. file + section — and offer the two resolution paths (change the docs first with a version bump, or change the request). Only proceed after the user decides.

Exception: if the docs contradict each other, the priority order in §2 decides *within* that domain — but a change that would move scope across doc boundaries (e.g. adding a firewall feature, a new screen, a new endpoint) always goes back to the user.

---

## 4. DO Rules

### 4.1 Structure
- **Follow the folder structure in ARCHITECTURE.md §4 exactly.** New files go where the tree says; no new top-level dirs without updating the doc first.
- One concern per module. Daemon logic stays in `crates/netwatch-*`; the Tauri crate stays thin (window/tray/link only); screens never import the WS client directly (ARCHITECTURE.md §6.1).

### 4.2 Reuse
- **Reuse existing components before writing new ones.** Check `app/src/lib/components/` first; a second button variant or a third table implementation is a bug.
- New UI components must be built from DESIGN.md §7 specs and vendored shadcn-svelte primitives, using only DESIGN.md §3–§6 tokens. Never inline hex values, raw font names, or magic spacing.
- Shared types go in `netwatch-common`; never copy types between crates or between TS and Rust by hand-editing two sources (ARCHITECTURE.md §4).

### 4.3 Code quality
- **Write small, focused functions.** One job per function; extract when a function grows past ~40 lines or a second branch nest appears.
- **Comment complex logic** — the *why* (trade-offs, references to doc sections), not the *what*. Cross-reference doc sections where it helps: `// PRD §6.1 #5: 90-day retention`.
- No `unwrap()`/`expect()` in daemon paths outside startup; catch panics at worker boundaries (ARCHITECTURE.md §7.1). TS: strict mode, no `any`.

### 4.4 Errors, loading, empty states
- **Handle errors, loading, and empty states for every screen and async action** — DESIGN.md §7/§8 define them; a view that renders only the happy path is incomplete.
- Map API error codes to the DESIGN.md states per ARCHITECTURE.md §7.3 / API.md §1.3 — never invent new error UI patterns.
- The daemon never dies on a failed task; degraded state + diagnostics event instead (ARCHITECTURE.md §7.1).

### 4.5 Docs stay in sync
- Changed an endpoint? Update API.md (+ `docs/api-contract.md`). Changed the schema? DATABASE.md first, then a migration. Changed tokens/components? DESIGN.md. The doc update is part of the same change, not a follow-up.

---

## 5. DO NOT Rules

- **Do not change the tech stack.** Tauri 2 / Svelte 5 / shadcn-svelte / ECharts / Rust + Tokio / Aya eBPF / SQLite are mandated (PRD §5, ARCHITECTURE.md §2). No substitutes, no additions "while we're at it".
- **Do not add new libraries, dependencies, or tools without asking first.** This includes npm packages, Rust crates, and cargo/npm subcommands. Ask with: what it is, why the existing stack can't do it, size/maintenance cost. Wait for a yes.
- **Do not modify the database schema without updating DATABASE.md.** No ad-hoc `ALTER TABLE`, no indexes without the named-query justification (DATABASE.md §5), no editing an applied migration — new forward-only migration files only (DATABASE.md §8).
- **Do not delete or rewrite working code unless the task asks for it.** Refactors are only in-scope when explicitly requested, or when they remove duplication against §4.2. Leave comments and structure you don't understand alone — ask.
- **Do not hardcode secrets.** No keys, tokens, or passwords in code, tests, or commits. The only secret in this product is the per-boot API token, auto-generated by the daemon (ARCHITECTURE.md §8.3); anything else that looks like a secret is a mistake.
- **Do not make the app call the network.** No telemetry, no analytics, no update pings, no font/CDN fetches (PRD non-goal; fonts are self-hosted, DESIGN.md §4.1). The UI's own Live view must be able to prove this.
- **Do not bypass the API boundary.** The UI never opens SQLite, never reads root-owned files, never shells out to the daemon. All data flows through the token-authenticated localhost API (ARCHITECTURE.md §5, API.md).
- **Do not write product code from REAPM.md patterns alone.** REAPM.md describes a *different* product (Go/Angular/firewall). Use it for context, not as an implementation source.
- **Do not weaken the rules in the docs to fit code you already wrote.** If the code drifted, the code is wrong (§4.5).

---

## 6. Workflow (every task)

1. **Plan first.** Before writing code: restate the goal in one or two sentences, list the files you will touch, and name the doc sections that govern the change (e.g. "DESIGN.md §7.2, API.md §2.7"). If the plan touches a source-of-truth file, say so explicitly and apply §3.
2. **Code second.** Implement in small steps. Run the relevant checks as you go: `cargo fmt && cargo clippy` for Rust, lint + `svelte-check` for the UI, tests for anything with a test harness (`tests/accuracy/` for accounting changes).
3. **Summarize third.** After coding, give a short summary: what changed (files + one line each), which docs (if any) you updated, what you verified (commands run / tests), and anything left open. Keep it brief — no essays.

Additional standing rules:
- Small, reviewable diffs. One task per change; no drive-by edits to unrelated files.
- Every commit message: Conventional Commits (ARCHITECTURE.md §8.1), referencing the task.
- If you cannot verify a change (no build, no tests runnable), say so in the summary — never claim verification you didn't perform.
- If a task turns out bigger than the plan predicted, stop, re-plan, and report — don't improvise scope.

---

## 7. Quick Checklist (before you say "done")

- [ ] Follows ARCHITECTURE.md §4 folder structure and §8 naming
- [ ] Reuses existing components/tokens (DESIGN.md) — no new primitives, no inline hex/fonts
- [ ] Errors, loading, empty states handled per DESIGN.md §7/§8
- [ ] New/changed endpoints match API.md (format, codes, pagination) and are in its traceability matrix
- [ ] Schema changes shipped as a new migration + DATABASE.md update
- [ ] No new dependencies without user approval; no secrets; no network calls
- [ ] Relevant source-of-truth file(s) updated in the same change, if the change touched their domain
- [ ] Short summary written: what changed, what was verified, what's open

---

*End of AGENTS.md. When in doubt: check the docs, then ask. Never guess scope.*
