//! Net Watch Pro eBPF collector programs (Phase C, C1–C3).
//!
//! Design validated by `spikes/ebpf-aya/` and `docs/spike-ebpf-report.md`:
//! cgroup_skb ingress/egress count L3 bytes into a per-cgroup map. The
//! aggregation key is the SOCKET's cgroup id (`bpf_skb_cgroup_id`) — never
//! `current`, which is softirq/IRQ context and names the wrong task.
//!
//! Observe-only: cgroup_skb returns CGROUP_SKB_ACCEPT (1); returning 0 drops
//! every packet (learned in the spike). No sock_ops program: identity comes
//! from the socket cgroup, which avoids the bpftune conflict at the root.

#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::__sk_buff,
    helpers::generated::{bpf_get_socket_cookie, bpf_skb_cgroup_id},
    macros::{cgroup_skb, map},
    maps::{Array, HashMap},
    programs::SkBuffContext,
    EbpfContext as _,
};

/// Must be nonzero: the kernel drops the packet when cgroup_skb returns 0.
const CGROUP_SKB_ACCEPT: i32 = 1;
const MAX_CGROUPS: u32 = 4096;

/// CONFIG[0] = ifindex of `lo`, written by the daemon before attach.
#[map]
static CONFIG: Array<u32> = Array::with_max_entries(1, 0);

/// Cumulative byte counters per socket cgroup id.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Counts {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub packets: u32,
}

#[map]
static BYTES: HashMap<u64, Counts> = HashMap::with_max_entries(MAX_CGROUPS, 0);

/// Per-socket flow identity and absolute counters, keyed by socket cookie.
/// `bpf_get_socket_cookie` is measured-legal in cgroup_skb (spike report §Sixth);
/// tuple fields come straight from `__sk_buff`. Supports IPv4 (`family == 2`)
/// and IPv6 (`family == 10`) (Decision D8 / Task 5).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Flow {
    pub cgroup_id: u64,
    pub local_ip: [u32; 4],
    pub remote_ip: [u32; 4],
    pub local_port: u32,
    pub remote_port: u32,
    pub protocol: u32,
    pub _pad: u32,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub packets_up: u32,
    pub packets_down: u32,
    pub family: u32,
    pub _pad2: u32,
}

#[map]
static FLOWS: HashMap<u64, Flow> = HashMap::with_max_entries(MAX_CGROUPS, 0);

/// STATS counters (indexes documented alongside the loader).
/// 0 = packets seen, 1 = loopback packets skipped, 2 = map insert failures.
#[map]
static STATS: Array<u64> = Array::with_max_entries(4, 0);

fn bump_stat(index: u32) {
    if let Some(ptr) = STATS.get_ptr_mut(index) {
        unsafe { *ptr += 1 };
    }
}

#[cgroup_skb(ingress)]
pub fn ingress(ctx: SkBuffContext) -> i32 {
    count_skb(&ctx, 0)
}

#[cgroup_skb(egress)]
pub fn egress(ctx: SkBuffContext) -> i32 {
    count_skb(&ctx, 1)
}

fn count_skb(ctx: &SkBuffContext, direction: u32) -> i32 {
    let skb_ptr = ctx.as_ptr();
    let skb = unsafe { &*(skb_ptr as *const __sk_buff) };
    let len = u64::from(ctx.len());

    // Loopback is filtered by ifindex, not by name or hardcoded value.
    let lo = CONFIG.get(0).copied().unwrap_or(0);
    if lo != 0 && (skb.ifindex == lo || skb.ingress_ifindex == lo) {
        bump_stat(1);
        return CGROUP_SKB_ACCEPT;
    }

    bump_stat(0);
    // The socket's cgroup — the app identity (Motrix + aria2c share one id).
    let cgroup_id = unsafe { bpf_skb_cgroup_id(skb_ptr as *mut __sk_buff) };
    if cgroup_id == 0 {
        // Not attributable to a cgroup; the daemon's Unattributed row covers it.
        return CGROUP_SKB_ACCEPT;
    }

    // Per-socket flow for Live/History. Supports AF_INET (2) and AF_INET6 (10).
    let cookie = unsafe { bpf_get_socket_cookie(skb_ptr) };
    if cookie != 0 && (skb.family == 2 || skb.family == 10) {
        let mut flow = match FLOWS.get_ptr_mut(&cookie) {
            Some(ptr) => unsafe { *ptr },
            None => Flow {
                cgroup_id,
                local_ip: [0; 4],
                remote_ip: [0; 4],
                local_port: 0,
                remote_port: 0,
                protocol: 0,
                _pad: 0,
                bytes_up: 0,
                bytes_down: 0,
                packets_up: 0,
                packets_down: 0,
                family: skb.family,
                _pad2: 0,
            },
        };
        flow.cgroup_id = cgroup_id;
        if skb.family == 2 {
            flow.local_ip = [skb.local_ip4, 0, 0, 0];
            flow.remote_ip = [skb.remote_ip4, 0, 0, 0];
        } else {
            flow.local_ip = skb.local_ip6;
            flow.remote_ip = skb.remote_ip6;
        }
        flow.local_port = skb.local_port;
        flow.remote_port = skb.remote_port;
        flow.protocol = skb.protocol;
        flow.family = skb.family;
        if direction == 0 {
            flow.bytes_down = flow.bytes_down.saturating_add(len);
            flow.packets_down = flow.packets_down.saturating_add(1);
        } else {
            flow.bytes_up = flow.bytes_up.saturating_add(len);
            flow.packets_up = flow.packets_up.saturating_add(1);
        }
        if FLOWS.insert(&cookie, &flow, 0).is_err() {
            bump_stat(2);
        }
    }

    let mut counts = match BYTES.get_ptr_mut(&cgroup_id) {
        Some(ptr) => unsafe { *ptr },
        None => Counts {
            rx_bytes: 0,
            tx_bytes: 0,
            packets: 0,
        },
    };
    if direction == 0 {
        counts.rx_bytes = counts.rx_bytes.saturating_add(len);
    } else {
        counts.tx_bytes = counts.tx_bytes.saturating_add(len);
    }
    counts.packets = counts.packets.saturating_add(1);
    if BYTES.insert(&cgroup_id, &counts, 0).is_err() {
        bump_stat(2); // map pressure; surfaced in Diagnostics
    }
    CGROUP_SKB_ACCEPT
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
