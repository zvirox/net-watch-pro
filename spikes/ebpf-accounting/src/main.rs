//! eBPF accounting spike — throwaway prototype (Part 3 of PLAN-FIX-INSTRUCTIONS.md).
//!
//! This is NOT product code. It lives outside the Cargo workspace and is never
//! imported by product crates. Its purpose is to answer Q1–Q10 and feed
//! `docs/spike-ebpf-report.md` before any production collector is written.
//!
//! To run: `cargo +nightly run --release` on a real VM (Arch/CachyOS, Ubuntu LTS,
//! Fedora). Requires root (CAP_BPF/CAP_PERFMON/CAP_NET_ADMIN) and cgroup v2.
//!
//! The eBPF program source below is a skeleton. Designs 1–3 (cgroup_skb cookie,
//! function hooks, hybrid) are implemented and compared on the VMs during the
//! spike; results go into docs/spike-ebpf-report.md.

use std::process::exit;

/// Placeholder entrypoint. The real implementation loads Aya eBPF programs
/// (socket/cgroup hooks), fills a cookie→(pid, cgroup) identity map at
/// connect/accept time, accumulates per-socket bytes in cgroup_skb ingress/
/// egress, and flushes deltas via a ring buffer ~1 s.
fn main() {
    eprintln!("netwatch-ebpf-spike: skeleton only — implement Designs 1–3 on target VMs");
    eprintln!("  Q1: hook set for TCP/UDP/QUIC/IPv4/IPv6 attribution");
    eprintln!("  Q2: attributed bytes vs interface counters gap");
    eprintln!("  Q3: CPU/RSS at 50/500 conns and high packet rate");
    eprintln!("  Q4: cgroup-path identity reliability (Flatpak/Snap/terminals)");
    eprintln!("  Q5: exec-time identity capture vs /proc reads");
    eprintln!("  Q6: required capabilities for load/attach/run");
    eprintln!("  Q7: Secure Boot / kernel lockdown on Ubuntu/Fedora");
    eprintln!("  Q8: map sizing, LRU eviction, ring-buffer drops, churn");
    eprintln!("  Q9: nightly + bpf-linker toolchain reality");
    eprintln!(" Q10: DB writes/s and index churn at proposed cadence");
    exit(0);
}
