//! eBPF accounting spike — per-app byte attribution measurement (Part 3).
//!
//! This is spike code, NOT product code. It lives outside the Cargo workspace.
//!
//! Build:   cargo +nightly build --release
//! Run:     sudo ./target/release/netwatch-ebpf-spike <workload>
//!
//! Requires root (CAP_BPF, CAP_NET_ADMIN, CAP_PERFMON) and cgroup v2.
//!
//! Measurements Q1–Q10 (see README.md):
//!   Q1: hook set for TCP/UDP/QUIC/IPv4/IPv6 attribution
//!   Q2: attributed bytes vs interface counters gap
//!   Q3: CPU/RSS at 50/500 conns and high packet rate
//!   Q4: cgroup-path identity reliability (Flatpak/Snap/terminals)
//!   Q5: exec-time identity capture vs /proc reads
//!   Q6: required capabilities for load/attach/run
//!   Q7: Secure Boot / kernel lockdown on Ubuntu/Fedora
//!   Q8: map sizing, LRU eviction, ring-buffer drops, churn
//!   Q9: nightly + bpf-linker toolchain reality
//!  Q10: DB writes/s and index churn at proposed cadence

use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: netwatch-ebpf-spike <workload>");
        eprintln!();
        eprintln!("Workloads:");
        eprintln!("  curl-download   Single-app download via curl");
        eprintln!("  curl-upload     Single-app upload via curl");
        eprintln!("  iperf3-tcp      iperf3 TCP throughput");
        eprintln!("  iperf3-udp      iperf3 UDP throughput");
        eprintln!("  short-curls     Loop of 5k short-lived curls");
        eprintln!("  idle-conns      500 idle connections");
        eprintln!("  wireguard       WireGuard/VPN on");
        eprintln!("  docker          Docker container making requests");
        eprintln!("  ipv6-only       IPv6-only traffic");
        eprintln!("  flatpak-app     Flatpak app traffic");
        eprintln!("  python-script   Python script making requests");
        eprintln!();
        eprintln!("Run as root: sudo ./target/release/netwatch-ebpf-spike <workload>");
        exit(1);
    }

    let workload = &args[1];
    eprintln!("netwatch-ebpf-spike: workload '{}' not yet implemented", workload);
    eprintln!("This is a skeleton — implement Designs 1–3 (cgroup_skb cookie,");
    eprintln!("function hooks, hybrid) on the target VM and record Q1–Q10 results.");
    eprintln!("Results go into docs/spike-ebpf-report.md.");
    exit(0);
}
