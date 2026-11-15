# eBPF Accounting Spike — Net Watch Pro

**Time box:** ~25 hours (1–2 weeks). Stop at the box and report, even if unfinished.

**Purpose:** Replace guesses about eBPF per-app byte accounting with measurements, per `PLAN-FIX-INSTRUCTIONS.md` Part 3. This directory is throwaway code — outside the Cargo workspace, never imported by product crates.

## Prerequisites (install before building)

1. **Nightly Rust toolchain** (via rustup — not currently installed in this environment):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup toolchain install nightly
   rustup component add --toolchain nightly rust-src llvm-tools-preview
   ```
2. **bpf-linker** (build-time tool for BTF relocations):
   ```bash
   cargo install bpf-linker   # or from https://github.com/aya-rs/bpf-linker
   ```
3. **Dev tools:** `iperf3`, `bpftool` (system packages: `iperf3`, `linux-tools` / `linux-tools-generic`)
4. **Target kernels (real VMs, not containers — containers share the host kernel):**
   - Arch/CachyOS current
   - Ubuntu LTS
   - Fedora current

## Build

```bash
cd spikes/ebpf-accounting
cargo +nightly build --release
```

## Questions the spike must answer (Q1–Q10)

| # | Question | How to measure |
|---|---|---|
| Q1 | Which hook set attributes bytes to the correct process/cgroup for TCP, UDP, QUIC, IPv4 and IPv6? | Compare Designs 1–3 below on workloads in §Workloads |
| Q2 | How close do attributed bytes get to interface counters, and what is the gap composed of? | Read `/sys/class/net/<if>/statistics/{rx,tx}_bytes` before/after each workload |
| Q3 | CPU and RSS overhead at 50 and 500 concurrent connections and at high packet rate | `pidstat`/`/proc` sampling, `iperf3` at line rate |
| Q4 | Is cgroup-path identity reliable on GNOME/KDE-launched apps, Flatpak, Snap, terminals? | Launch a browser, a Flatpak app, a script; record cgroup paths |
| Q5 | Can the daemon get process identity without reading `/proc/<pid>/exe` of other users? | Exec-time capture (`sched_process_exec` tracepoint) vs `/proc` reads under bounded capability set |
| Q6 | What capabilities are *actually* required for load, attach and run? | Run with bounded set (`CAP_BPF`, `CAP_PERFMON`, `CAP_NET_ADMIN`); record failures |
| Q7 | Does anything break under Secure Boot / kernel lockdown on Ubuntu and Fedora? | Boot VMs with Secure Boot on; load and run |
| Q8 | Behaviour under churn: map sizing, LRU eviction, ring-buffer drops, short-lived processes | Loop of 5k short `curl`s; watch drop counters |
| Q9 | Toolchain reality: nightly requirement, `bpf-linker` install, build time, reproducibility, CI feasibility | Document exact commands and versions |
| Q10 | Write load: DB writes/s and index churn with N open connections under the proposed cadence | Simulate with fixture collector against SQLite |

## Candidate designs to implement and compare

### Design 1 — cgroup_skb counters keyed by socket cookie
Count bytes in `cgroup_skb` ingress/egress per socket cookie; fill a cookie→(pid, cgroup) map at connect/accept time via cgroup `sock`/`sock_addr`/`sockops` hooks. Counts L3 bytes (closer to interface counters). **[HYPOTHESIS]** Ingress runs in softirq context, so `current` PID is unreliable — hence the cookie map.

### Design 2 — function hooks
kprobe/fentry on `tcp_sendmsg`, `tcp_cleanup_rbuf` (or `tcp_recvmsg`), `udp_sendmsg`, `udp_recvmsg` with socket pointer and process context. Counts payload bytes, not wire bytes. **[HYPOTHESIS]** fentry has newer-kernel requirements on arm64.

### Design 3 — hybrid
Design 1 for byte counts; Design 2 only to fill identity where the cookie map misses.

Choose with a decision table (accuracy, overhead, kernel floor, complexity, failure modes). Do not pick on elegance.

## Workloads

For each workload, record: interface delta, attributed total, per-app split, unattributed remainder:

1. Single-app download and upload (`curl`)
2. `iperf3` TCP and UDP
3. Browser doing QUIC/HTTP3
4. Loop of 5k short-lived `curl`s
5. 500 idle connections
6. WireGuard/VPN on
7. Docker container making requests
8. IPv6-only traffic
9. Flatpak app
10. Python script making requests

## Deliverables

1. **This directory** — spike source, README with exact build/run steps and versions.
2. **`docs/spike-ebpf-report.md`** — answers to Q1–Q10, measurement tables, chosen design, rejected designs with reasons, and a list of product claims to soften or drop.
3. **Proposed `[HYPOTHESIS]` → `[VERIFIED: spike-ebpf-report §n]` / `[REFUTED]` tag updates.**

## Success / stop criteria

- **Proceed** if, on a single-app workload, attributed bytes are within ~5% of that app's real traffic, and attributed + unattributed reconciles with interface counters within ~2%, at under the CPU/RSS budget (≤3% CPU, ≤150 MB RSS), on all three VM kernels.
- **Re-scope instead** if UDP/QUIC or short-lived-process attribution cannot be made reliable: change the PRD to label confidence per row and downgrade the promise, rather than building on a shaky base.
- Report to the owner at the time box regardless of outcome.

## Notes

- This environment does not have rustup/nightly or VMs, so the spike **cannot be executed here**. The scaffolding (Cargo.toml, rust-toolchain.toml) is ready; a developer with rustup and VM access should run it.
- All kernel/eBPF claims in product docs remain `[HYPOTHESIS]` until the spike report verifies them (see `docs/decisions.md`, `AGENTS.md` §4.6).
