# SETUP.md — Developer Setup Guide

**Version:** 2.0
**Last Updated:** 2026-11-25
**Applies to:** Local development and CI on Linux (`x86_64` / `aarch64`)

---

## 1. Prerequisites

| Tool | Requirement | Notes |
|---|---|---|
| Rust toolchain | Stable (workspace `edition = "2024"`, CI pins `1.99.1`) | `rustup component add clippy rustfmt` |
| eBPF build tools *(only for `cargo run -p xtask -- bpf`)* | `nightly` + `rust-src`, `bpf-linker`, `clang`/`llvm` | Optional for non-eBPF builds; daemon runs in `no_collector` degraded mode without `netwatch_ebpf.o` |
| Node.js & npm | Node `>= 20`, `npm >= 10` | Manages `app/` dependencies |
| Python | `python3 >= 3.10` | `tests/a11y/contrast.py`, `tests/accuracy/`, `tests/integration/` |
| Tauri 2 / GTK system libs (Arch / CachyOS) | `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`, `openssl`, `polkit` | Ubuntu: `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev libssl-dev` |

---

## 2. Quick Start

```bash
# 1. Clone and install frontend dependencies
git clone https://github.com/zvirox/net-watch-pro.git
cd net-watch-pro
npm --prefix app ci

# 2. Verify frontend design gate, contrast pairs, types, lint, unit tests, and build
python3 tests/a11y/contrast.py
npm --prefix app run check:linear
npm --prefix app run typecheck
npm --prefix app run lint
npm --prefix app test
npm --prefix app run build

# 3. Verify Rust workspace (build daemon + app, clippy, unit & integration tests)
cargo build -p netwatchd -p netwatch-app
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

> [!NOTE]
> `netwatch-integration-tests --test restart_persistence` spawns `target/debug/netwatchd`. Run `cargo build -p netwatchd` before running `cargo test --workspace`.

---

## 3. Running Locally

### Frontend UI Preview (Demo Mode)
Runs the full locked Linear UI in any browser without `netwatchd` or root privileges:

```bash
npm --prefix app run dev
```

### Tauri Desktop App + Daemon
1. Optional eBPF collector build:
   ```bash
   cargo run -p xtask -- bpf
   ```
2. Build & launch Tauri window (`1280x800`, custom titlebar):
   ```bash
   cargo build -p netwatchd
   cargo run -p netwatch-app
   ```
   Or build release native packages locally with `nfpm` using `packaging/nfpm.yaml` (see `.github/workflows/release.yml`).

---

## 4. Verification & Troubleshooting

- **`no target/netwatch_ebpf.o` compile warning:** Expected unless `cargo run -p xtask -- bpf` was run. The daemon starts cleanly in `no_collector` state and reports it honestly in Diagnostics.
- **Tauri tray / WebKit build errors:** Confirm `webkit2gtk-4.1` and `libayatana-appindicator` dev headers are installed.
- **Linear UI gate (`check:linear`) failure:** The Linear design system (`DESIGN.md` v3.0) is final and locked; remove any blur, translucency, box-shadow in dark mode, glow, or hardcoded off-token colors.
