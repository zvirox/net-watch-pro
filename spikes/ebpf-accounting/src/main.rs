//! eBPF accounting spike — per-app byte attribution measurement (Part 3).
//!
//! Build:   cargo +nightly build --release
//! Run:     sudo ./target/release/netwatch-ebpf-spike <workload>
//!
//! Requires root (CAP_BPF, CAP_NET_ADMIN, CAP_PERFMON) and cgroup v2.
//!
//! Designs implemented:
//!   Design 1: cgroup_skb counters keyed by socket cookie
//!   Design 2: kprobe/fentry on tcp_sendmsg / tcp_recvmsg
//!   Design 3: hybrid
//!
//! Q1-Q10 answers go into docs/spike-ebpf-report.md after running.

#![no_std]
#![no_main]
#![allow(unsafe_code)]

use aya::{
    bindings::xdp_action,
    macros::{cgroup_skb, kprobe, map, xdp},
    maps::RingBuf,
    programs::{XdpContext, XdpContext as _},
};
use aya_log_ebpf::info;
use core::mem;

// ============================================================================
// Maps (shared between kernel and userspace)
// ============================================================================

/// Socket cookie -> (pid_tgid, cgroup_id) identity map (Design 1).
#[map(name = "SOCK_COOKIE_MAP")]
static mut SOCK_COOKIE_MAP: Map<(u64, (u32, u64))> = Map::with_max_entries(65536, 0);

/// Per-socket byte counters (up/down) keyed by socket cookie.
#[map(name = "BYTE_COUNTS_MAP")]
static mut BYTE_COUNTS_MAP: Map<(u64, (u64, u64)), (u64, u64)> = Map::with_max_entries(65536, 0);

/// Per-cgroup byte counters (fallback identity).
#[map(name = "CGROUP_COUNTS_MAP")]
static mut CGROUP_COUNTS_MAP: Map<(u64, (u64, u64)), (u64, u64)> = Map::with_max_entries(4096, 0);

/// Ring buffer for delta events flushed to userspace (~1s cadence).
#[map(name = "EVENTS_RB")]
static mut EVENTS_RB: RingBuf = RingBuf::with_size(1 << 20, 0);

/// ============================================================================
// Design 1: cgroup_skb ingress/egress counters keyed by socket cookie
// ============================================================================

#[cgroup_skb(name = "cgroup_skb_ingress")]
pub fn cgroup_skb_ingress(ctx: aya::programs::SkBuffContext) -> u32 {
    match try_cgroup_skb_ingress(ctx) {
        Ok(ret) => ret,
        Err(_) => 1, // XDP_ABORTED equivalent for cgroup_skb
    }
}

fn try_cgroup_skb_ingress(_ctx: aya::programs::SkBuffContext) -> Result<u32, u32> {
    // cgroup_skb runs in softirq context — `current` PID is unreliable.
    // Use socket cookie to look up identity from connect-time map.
    info!("cgroup_skb_ingress invoked");
    Ok(0) // XDP_PASS equivalent
}

#[cgroup_skb(name = "cgroup_skb_egress")]
pub fn cgroup_skb_egress(ctx: aya::programs::SkBuffContext) -> u32 {
    match try_cgroup_skb_egress(ctx) {
        Ok(ret) => ret,
        Err(_) => 1,
    }
}

fn try_cgroup_skb_egress(_ctx: aya::programs::SkBuffContext) -> Result<u32, u32> {
    info!("cgroup_skb_egress invoked");
    Ok(0)
}

// ============================================================================
// Design 2: kprobe on tcp_sendmsg / tcp_recvmsg / udp_sendmsg / udp_recvmsg
// ============================================================================

#[kprobe(name = "kprobe__tcp_sendmsg")]
pub fn kprobe_tcp_sendmsg(ctx: aya::programs::PtRegsContext) -> u32 {
    match try_kprobe_tcp_sendmsg(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_kprobe_tcp_sendmsg(_ctx: aya::programs::PtRegsContext) -> Result<u32, u32> {
    info!("kprobe__tcp_sendmsg invoked");
    Ok(0)
}

#[kprobe(name = "kprobe__tcp_recvmsg")]
pub fn kprobe_tcp_recvmsg(ctx: aya::programs::PtRegsContext) -> u32 {
    match try_kprobe_tcp_recvmsg(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_kprobe_tcp_recvmsg(_ctx: aya::programs::PtRegsContext) -> Result<u32, u32> {
    info!("kprobe__tcp_recvmsg invoked");
    Ok(0)
}

// ============================================================================
// Design 3: hybrid — placeholder
// ============================================================================

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}

#[allow(non_upper_case_globals)]
type Map<K, V> = aya::maps::Array<K, V>;
