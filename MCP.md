# MCP.md — AI Assistant Tooling via Model Context Protocol

**Version:** 1.1 · **Date:** 2026-11-15  
**Related:** `AGENTS.md` (assistant behavior rulebook) · `ARCHITECTURE.md` §8.3 (env vars) · `DATABASE.md` (schema the DB tools may read) · `docs/decisions.md` (owner decisions D1–D7)

**Version history:** 1.1 (2026-11-15): no product-transport changes (MCP is dev tooling only); version bumped for consistency with plan-fix effort.

> **Important distinction:** the *product* Net Watch Pro makes zero outbound connections (PRD, AGENTS.md §5). MCP servers here are **developer tooling for the assistant working on the repo**, not part of the product — they never ship in the app, never run as root, and never touch `/var/lib/netwatch-pro` except through the explicitly allowed read-only DB connection below.

---

## 1. Principles

1. **Least privilege per server** — every server starts from a documented allowed-tool list; everything not listed is blocked in client config, not just "asked not to".
2. **Read-only by default** — write-capable tools stay disabled until a task explicitly needs them; destructive tools are always blocked in config and only reachable via an explicit human-approved temporary change to this file.
3. **Confirm before destructive actions** — writes (file edits, issue creation, DB mutations) require the assistant to state intent first and wait for user confirmation (AGENTS.md §3/§6).
4. **No real keys in this file** — every credential is an environment-variable placeholder resolved by the MCP client's env expansion (`${VAR}`); the actual values live outside the repo (shell env / secret manager), `.gitignore`d.
5. **Scope every path/token** — filesystem roots pinned to this repo; GitHub token scoped to this repository only (fine-grained PAT); DB connection pointed at a **snapshot/copy**, never the daemon's live WAL file while it runs.

---

## 2. Recommended Servers (and why)

| Server | Why this project needs it | Priority |
|---|---|---|
| **filesystem** | Repo-wide reading/searching of the 9 spec files + code once scaffolding starts, with paths pinned to the project root | Core |
| **sqlite** | Inspect schema/seed/accuracy data against `netwatch.db` copies; verify DATABASE.md ↔ schema conformance | Core (post-P2.4) |
| **github** | Issues, PRs, release tracking for the PLAN.md/CHECKLIST.md release flow | Recommended |
| **browser (Playwright)** | Drive the Tauri webview/dev URL to verify DESIGN.md §7/§8 states (loading/empty/error) against the running app | Recommended (post-P0.4) |
| **docs / context7** | Up-to-date library docs for Tauri 2, Svelte 5, Aya, sqlx — reduces hallucinated APIs | Optional |

Explicitly **not recommended** here (one-line reasons): generic shell/exec servers (the assistant already has a sandboxed shell; a second execution path doubles risk), Slack/Google servers (no workflow need), anything requiring broad account scopes.

---

## 3. Configuration Snippets

Standard `mcpServers` JSON — works in any MCP-compatible client (OpenCode, Claude Desktop, etc.). Merge into your client's config; do **not** commit real values.

### 3.1 filesystem

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y", "@modelcontextprotocol/server-filesystem",
        "${PROJECT_ROOT}"
      ]
    }
  }
}
```
- `${PROJECT_ROOT}` = absolute path to `/home/zvirox/Projects/net-watch-pro` (set in the client env, never hardcoded here).
- **Allowed tools:** `read_file`, `read_multiple_files`, `list_directory`, `directory_tree`, `search_files`, `get_file_info`, `list_allowed_directories`.
- **Blocked tools:** `write_file`, `edit_file`, `create_directory`, `move_file`, `copy_file`, `list_directory_with_sizes` *(if exposed)* — the assistant writes through its own reviewed tooling with user visibility, not a side channel.
- **Why this scope:** one root only; reads are safe; writes belong to the audited editor workflow (AGENTS.md §6 small-diffs rule).

### 3.2 sqlite

```json
{
  "mcpServers": {
    "sqlite-netwatch": {
      "command": "npx",
      "args": [
        "-y", "@modelcontextprotocol/server-sqlite",
        "--db-path", "${NETWATCH_DB_COPY}"
      ]
    }
  }
}
```
- `${NETWATCH_DB_COPY}` = path to a **snapshot copy** (e.g. `/tmp/opencode/netwatch-inspect.db` produced via `VACUUM INTO` — DATABASE.md §6); never the live WAL pair.
- **Allowed tools:** `read_query`, `list_tables`, `describe_table`, `schema_graph`(if exposed).
- **Blocked tools:** `write_query`, `create_table`, `append_insight` — schema changes must go through sqlx migrations + DATABASE.md review (AGENTS.md §5), never through a side door.
- **Why:** verification and debugging read paths only; the daemon owns writes (DATABASE.md §6 access rules).

### 3.3 github

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "${GITHUB_MCP_PAT}"
      }
    }
  }
}
```
- `${GITHUB_MCP_PAT}` = **fine-grained PAT, this repository only**, scopes: *Contents: read, Issues: read, Pull requests: read*. No `Administration`, no `Delete`, no workflows scope.
- **Allowed tools:** `get_issue`, `list_issues`, `search_issues`, `get_pull_request`, `list_pull_requests`, `get_file_contents`, `list_commits`, `search_code`.
- **Blocked by default in config:** `create_issue`, `create_pull_request`, `add_issue_comment`, `merge_pull_request`, `delete_*`, anything `push`/admin — enabling these is a temporary, user-approved edit of this file for the duration of a task.
- **Why:** tracking releases/issues/PRs against PLAN.md phases; write actions flow through the human's git/gh workflow so every change is attributable.

### 3.4 browser (Playwright)

```json
{
  "mcpServers": {
    "browser": {
      "command": "npx",
      "args": ["-y", "@playwright/mcp@latest", "--viewport-size=1280,800"]
    }
  }
}
```
- Targets the Tauri dev webview URL (`tauri dev` serves the frontend; ARCHITECTURE.md §4) — the daemon API stays out of reach: the browser profile must not hold the API token (the WS client fetches it via the Tauri helper, not the browser).
- **Allowed tools:** `browser_navigate`, `browser_snapshot`, `browser_click`, `browser_type`, `browser_console_messages`, `browser_take_screenshot`, `browser_resize`.
- **Blocked tools:** `browser_file_upload` (except into onboarding test fixtures), download-related tools by default.
- **Why:** state verification (loading/empty/error/focus rings) against DESIGN.md §7/§8 without touching the daemon.

### 3.5 docs (context7)

```json
{
  "mcpServers": {
    "docs": {
      "command": "npx",
      "args": ["-y", "@upstash/context7-mcp@latest"]
    }
  }
}
```
- **Allowed tools:** `resolve-library-id`, `get-library-docs`. **Blocked:** none exist beyond those two.
- **Why:** pinned-versions docs for Tauri 2 / Svelte 5 runes / Aya / sqlx — the mandated stack's APIs are recent and change fast.

---

## 4. Permission & Safety Rules (all servers)

1. **Default posture: read-only.** A server not needed for the current task should be disabled in the client, not just unused.
2. **Write actions need explicit confirmation.** The assistant states: what, where, consequence — then waits (AGENTS.md §3). No speculative writes "to test".
3. **Destructive = blocked in config**, not blocked by politeness. Deletes, schema mutations, force-pushes, token-carrying requests are unreachable until this file is edited with the user's approval.
4. **Secrets hygiene:** placeholders only in this file; real values never quoted back into chat; if a secret appears in a tool result, the assistant flags it and the value is rotated — treated as leaked (AGENTS.md §5).
5. **Zero-outbound boundary:** GitHub/docs servers may touch the network **only when the user asks for a GitHub/docs task**; during product-feature work (F1–F10) they stay disabled so the assistant never normalizes the app phoning home.
6. **Snapshot rule for DB tools:** any schema/seed verification happens on `VACUUM INTO` copies; concurrent read of the live WAL is forbidden (locking + privacy).
7. **Every server gets a name prefix** in the client (e.g. `github:`) so the assistant's tool calls are auditable in the transcript.

---

## 5. Connection Tests (run once per setup, and after config changes)

| Server | Test prompt/command | Expected result | Fail → §6 |
|---|---|---|---|
| filesystem | "List the files in the project root and read the first 5 lines of AGENTS.md." | 9 spec files listed; AGENTS.md header shown; writes refused if attempted | A |
| sqlite | "List tables and describe the `connections` table." (against a fresh `VACUUM INTO` copy) | 8 tables from DATABASE.md §2; columns match §2.4; `write_query` refused | B |
| github | "Search issues in this repo and read PR #1 metadata." | Results returned; create-issue attempt refused; check PAT expiry if 401 | C |
| browser | "Navigate to the dev URL and take a snapshot; report console messages." | Themed window renders (DESIGN.md tokens); no console errors | D |
| docs | "Fetch the Tauri 2 IPC docs via docs server." | Relevant current docs returned | E |

Record pass/fail in the PR that changes MCP.md or the client config.

---

## 6. Troubleshooting Common Connection Errors

| Symptom | Likely cause | Fix |
|---|---|---|
| **A:** Server starts, tools all blocked | Client ignore-list not cleared after config change | Re-check client's blocked/allowed lists against §3; restart client. |
| **A:** `ENOENT` / npx fails | Node ≥ 20 not on PATH, or npx cache corrupt | `node --version`; run `npx -y @modelcontextprotocol/server-filesystem --help` manually. |
| **A:** "Access denied — path outside allowed directories" | `${PROJECT_ROOT}` empty (env not expanded) or trailing slash mismatch | Set the var in the *client's* env block; use one absolute path, no `~`. |
| **B:** "database is locked" | Pointed at the live WAL db while daemon runs | Never do this; make a copy with `sqlite3 src "VACUUM INTO '/tmp/copy.db'"` (DATABASE.md §6 snapshot rule). |
| **B:** "no such table" | Copy predates migrations, or wrong file (`netwatch.db-wal` passed instead of `.db`) | Copy the `.db` after a clean shutdown; re-run migrations on the copy if needed. |
| **C:** 401 `Bad credentials` | PAT expired/revoked, or env var not expanded (literal `${GITHUB_MCP_PAT}` sent) | Regenerate fine-grained PAT; verify expansion by checking the server's startup logs show the header present (not the value). |
| **C:** 403/404 on repos you can see in gh CLI | PAT not scoped to this repository, or classic PAT missing `repo` | Recreate fine-grained PAT with this repo selected + read scopes (§3.3). |
| **C:** Tool list empty | Server downgraded to unauthenticated mode after 401 | Fix token first; restart server. |
| **D:** Browser shows blank/white page | Vite dev server not running, or wrong port | Start `tauri dev` first; confirm the URL matches the dev server output. |
| **D:** Tools hang | Playwright downloading browsers on first run | Run `npx playwright install chromium` once; allow it to finish. |
| **D:** Can't reach daemon API from browser | **Expected** — token isn't in browser storage by design (§3.4) | Test API separately with curl + token; don't bypass the design. |
| **E:** "Library not found" | Wrong library id casing | Use `resolve-library-id` first, then the returned id verbatim. |
| Any: server silently absent | JSON syntax / duplicate `mcpServers` keys after merge | Validate JSON; keep one `mcpServers` object; check client logs. |
| Any: env var sent literally (`${VAR}`) | Client doesn't expand env in args/env blocks | Set values via the client's env passthrough or wrapper script; never paste real values here. |

---

## 7. Governance

- This file changes with a version bump like all source-of-truth docs (AGENTS.md §2); allow-list edits are user-approved changes, not assistant edits.
- Quarterly review: remove servers no longer used; re-check PAT scopes/expiry; re-run §5 tests.
- MCP servers are **out of the product's threat model** by design — if any of this tooling ever ships inside Net Watch Pro, that's a PRD change requiring the AGENTS.md §3 stop-and-ask.

---

*End of MCP.md. Placeholders only: `${PROJECT_ROOT}`, `${NETWATCH_DB_COPY}`, `${GITHUB_MCP_PAT}` — real values never enter this repo.*
