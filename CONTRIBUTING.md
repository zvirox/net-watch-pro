# Contributing to Net Watch Pro

Thank you for contributing! This project's rulebook — **[`AGENTS.md`](AGENTS.md)** —
applies to **human and AI contributors alike**. Read it first; it is short and
binding. Highlights below.

## Ground rules

1. **The docs are the contract.** Source of truth in priority order:
   `PRD.md` → `DESIGN.md` → `ARCHITECTURE.md` → `DATABASE.md` → `API.md`
   (`AGENTS.md` §2). Code that drifts from them is wrong.
2. **Conflicts stop at the user/maintainer.** If your change would contradict a
   source-of-truth file, open an issue describing the conflict — do not pick a
   side silently (`AGENTS.md` §3).
3. **Scope discipline.** Build what `PRD.md` §6.1 (MVP) lists — not V2, not
   Later, not the post-1.0 firewall. `FEATURES.md` §11 fixes the build order
   (one feature at a time, tests green before the next).
4. **No new dependencies without maintainer approval** — npm packages, Rust
   crates, CI tooling (propose: what / why the stack can't do it / cost).
5. **No secrets, no network calls in the product.** Ever (see `SECURITY.md`).

## Workflow

1. **Plan first** — restate the goal, list files, name the governing doc
   sections (`AGENTS.md` §6).
2. **Code second** — small focused functions, reuse existing components
   (`DESIGN.md` tokens only), handle loading/empty/error states everywhere,
   comment *why* not *what*.
3. **Test third** — tests ship in the same change as the code
   (`TESTING.md` §8 gate): unit + integration/e2e rows from the feature's
   `TESTING.md` §4 table.
4. **Summarize** — what changed, what you verified (commands/tests), what's
   open.

## Local gate (run before every PR)

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd app && npm run lint && npm run check && npm run test
```

CI must be green; PRs that break the gate will not be reviewed further.

## Commit & PR conventions

- **Conventional Commits** (`ARCHITECTURE.md` §8.1):
  `feat(dashboard): …`, `fix(pruner): …`, `docs(api): …`, `chore: …`.
- One task per change; small reviewable diffs; no drive-by edits.
- **Doc sync is part of the change** (`AGENTS.md` §4.5): endpoint change →
  `API.md` + `docs/api-contract.md`; schema change → `DATABASE.md` + a new
  forward-only migration; UI token/component change → `DESIGN.md`.
- PR description: link the task ID (`P*.**` in `PLAN.md` / `CHECKLIST.md`),
  list doc files touched, paste the verification output.

## Branches

- `main` — always runnable; protected; conventional commits only.
- Feature branches: `<type>/<short-name>` (e.g. `feat/live-table`).
- Release tags: `vX.Y.Z-alpha.N` / `-beta.N` / stable per `PLAN.md` §1.

## Licensing of contributions

The project is dual-licensed **AGPL-3.0-or-later + commercial**
(`README.md` §License). By contributing you agree your work is released under
both options (the same terms as in `LICENSE` + the commercial license the
maintainers offer). If your employer requires a copyright disclaimer, get it
signed before contributing.

## AI-assisted contributions

Welcome, with the same rules: the assistant must follow `AGENTS.md` §4–§7
(plan → code → summary; checklist before "done"; no unverified claims). The PR
description should state which parts were AI-assisted.

## Questions

Open a GitHub Discussion/Issue with the `question` label. For security
topics, use the private channel in `SECURITY.md` — never public issues.
