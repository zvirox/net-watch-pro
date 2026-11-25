# CONTRIBUTING.md — Contributing to Net Watch Pro

We welcome contributions to Net Watch Pro! Because it runs a privileged Linux eBPF daemon (`netwatchd`) and ships an offline-first desktop UI (`app/`), please follow these standards before opening a pull request.

---

## Before You Start

1. Read `AGENTS.md`, `DESIGN.md`, and `docs/decisions.md` before changing behavior or UI.
2. **UI Style Lock (`DESIGN.md` v3.0):** The **Linear Design System** (`#050505` / `#111213` dark, `#F3F4F6` / `#FFFFFF` light, solid surfaces, zero blur/glow/dark-shadow) is **final and locked**. PRs introducing glassmorphism, translucency, glows, or unapproved visual restyling will fail `npm --prefix app run check:linear`.
3. **Never touch packet payloads or add network calls** from `netwatchd` or `app/`. Privacy invariants in `PRD.md` §10 and `SECURITY.md` are non-negotiable.
4. Check `CHECKLIST.md` and `docs/ROADMAP.md` to confirm whether your change belongs to current stabilization/gates (`v0.9.3`–`v1.0.0`) or post-1.0 releases.

---

## Development Checks (Required Before PR)

Run the full verification suite locally before submitting:

```bash
# 1. Frontend & Linear design gates
python3 tests/a11y/contrast.py
npm --prefix app run check:linear
npm --prefix app run typecheck
npm --prefix app run lint
npm --prefix app test
npm --prefix app run build

# 2. Rust workspace (build netwatchd first so restart_persistence integration test finds it)
cargo fmt --check
cargo build -p netwatchd -p netwatch-app
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Scope & Commit Discipline

- Keep commits small, focused, and tied to a single feature, fix, or doc update.
- Preserve existing comments and docstrings unless the code they document changes.
- Update `CHANGELOG.md` and `CHECKLIST.md` only when verified by real command output or host evidence.
- Any change to eBPF programs, Polkit helpers (`packaging/polkit/`), or `SO_PEERCRED` peer checks (`crates/netwatch-api/src/peer.rs`) must update `AUTH_SECURITY.md` and `SECURITY.md` in the same PR.
