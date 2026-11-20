//! Phase B spike — eBPF programs (Aya), throwaway measurement code.
//!
//! Design 1: cgroup_skb ingress/egress count L3 bytes per socket cookie,
//! but the AGGREGATION KEY is the socket's cgroup id read with
//! `bpf_skb_cgroup_id` — not `current` (softirq/IRQ thread, §Twelfth run).
//! Loopback traffic is tagged and reported separately.
//! pid/comm are DEBUG ONLY and must never be used as an app identity.
//!
//! Nothing here is wired into netwatchd. The owner runs this on real root and
//! records measurements in docs/spike-ebpf-report.md (LIVE-THROUGHPUT-PLAN §5).

#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::{
        __sk_buff, BPF_SOCK_OPS_ACTIVE_ESTABLISHED_CB, BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB,
    },
    helpers::generated::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_socket_cookie, bpf_sk_cgroup_id,
        bpf_skb_cgroup_id,
    },
    macros::{cgroup_skb, map, sock_ops},
    maps::{Array, HashMap, RingBuf},
    programs::{SkBuffContext, SockOpsContext},
    EbpfContext as _,
};

const MAX_ENTRIES: u32 = 8192;
const RING_BYTES: u32 = 256 * 1024;
const SRC_CGROUP_SKB: u32 = 1;
const SRC_SOCK_OPS: u32 = 3;
/// Loopback bytes, reported separately by userspace (never compared to NIC).
const SRC_LOOPBACK: u32 = 4;
/// cgroup_skb verdict: the kernel DROPS the packet when the program returns
/// 0 and accepts it on any nonzero value; nonzero accepts (learned 2026-11-20).
const CGROUP_SKB_ACCEPT: i32 = 1;

/// Configuration written by userspace before attach: index 0 = `lo` ifindex.
#[map]
static CONFIG: Array<u32> = Array::with_max_entries(1, 0);

/// Per-cgroup debug metadata. `pid`/`comm` come from `current` and are
/// UNRELIABLE (softirq) — recorded for debugging, never as the identity key.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Meta {
    pub pid: u32,
    pub tid: u32,
    pub comm: [u8; 16],
    pub ingress_hits: u32,
    pub egress_hits: u32,
    pub identity_hits: u32,
}

/// Cumulative counters per cgroup id (useful when the ring buffer drops).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Counts {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub packets: u32,
}

/// One ring-buffer event. Layout must match `Delta` in the userspace crate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Delta {
    /// Socket cookie — debug only (distinct socket count).
    pub cookie: u64,
    /// Aggregate key: the socket's cgroup id, from bpf_skb/bpf_sk_cgroup_id.
    pub cgroup_id: u64,
    /// Debug only: 'current' task in softirq — NOT the app identity.
    pub pid: u32,
    pub tid: u32,
    pub delta_bytes: u64,
    pub direction: u32, // 0 = rx/ingress, 1 = tx/egress
    pub source: u32,    // SRC_CGROUP_SKB, SRC_SOCK_OPS or SRC_LOOPBACK
    pub comm: [u8; 16],
}

#[map]
static META: HashMap<u64, Meta> = HashMap::with_max_entries(MAX_ENTRIES, 0);
#[map]
static COUNTS: HashMap<u64, Counts> = HashMap::with_max_entries(MAX_ENTRIES, 0);
#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(RING_BYTES, 0);

fn empty_meta() -> Meta {
    Meta {
        pid: 0,
        tid: 0,
        comm: [0u8; 16],
        ingress_hits: 0,
        egress_hits: 0,
        identity_hits: 0,
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

    // Loopback is injected from userspace so we never hardcode "ifindex 1".
    let lo_ifindex = CONFIG.get(0).copied().unwrap_or(0);
    let is_loopback =
        lo_ifindex != 0 && (skb.ifindex == lo_ifindex || skb.ingress_ifindex == lo_ifindex);

    // THE identity source: the cgroup of the socket that owns this skb.
    // `bpf_skb_cgroup_id` works in softirq, unlike `current`.
    let cgroup_id = unsafe { bpf_skb_cgroup_id(skb_ptr as *mut __sk_buff) };
    let cookie = unsafe { bpf_get_socket_cookie(skb_ptr) };
    let len = u64::from(ctx.len());
    let source = if is_loopback {
        SRC_LOOPBACK
    } else {
        SRC_CGROUP_SKB
    };

    if !is_loopback {
        let mut counts = match COUNTS.get_ptr_mut(&cgroup_id) {
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
        let _ = COUNTS.insert(&cgroup_id, &counts, 0);
    }

    // Debug-only pid/comm from 'current' — softirq, do not trust as identity.
    let mut meta = match META.get_ptr_mut(&cgroup_id) {
        Some(ptr) => unsafe { *ptr },
        None => empty_meta(),
    };
    if meta.pid == 0 {
        let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
        let pid = (pid_tgid >> 32) as u32;
        if pid != 0 {
            meta.pid = pid;
            meta.tid = pid_tgid as u32;
            let _ = unsafe {
                bpf_get_current_comm(meta.comm.as_mut_ptr() as *mut core::ffi::c_void, 16)
            };
        }
    }
    if direction == 0 {
        meta.ingress_hits = meta.ingress_hits.saturating_add(1);
    } else {
        meta.egress_hits = meta.egress_hits.saturating_add(1);
    }
    let _ = META.insert(&cgroup_id, &meta, 0);

    let _ = EVENTS.output(
        &Delta {
            cookie,
            cgroup_id,
            pid: meta.pid,
            tid: meta.tid,
            delta_bytes: len,
            direction,
            source,
            comm: meta.comm,
        },
        0,
    );
    CGROUP_SKB_ACCEPT
}

/// sock_ops kept as a secondary source: `bpf_sk_cgroup_id` on the socket is a
/// reliable cgroup id even when `current` is softirq. It never calls set_reply.
#[sock_ops]
pub fn identity(ctx: SockOpsContext) -> u32 {
    let op = ctx.op();
    if op != BPF_SOCK_OPS_ACTIVE_ESTABLISHED_CB && op != BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB {
        return 0;
    }
    let sk_ptr = ctx.as_ptr();
    let cgroup_id = unsafe { bpf_sk_cgroup_id(sk_ptr) };
    if cgroup_id == 0 {
        return 0;
    }
    let cookie = unsafe { bpf_get_socket_cookie(sk_ptr) };
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };

    let mut meta = match META.get_ptr_mut(&cgroup_id) {
        Some(ptr) => unsafe { *ptr },
        None => empty_meta(),
    };
    // pid/comm retained as DEBUG ONLY (softirq may be the IRQ thread).
    if meta.pid == 0 {
        meta.pid = (pid_tgid >> 32) as u32;
        meta.tid = pid_tgid as u32;
        let _ =
            unsafe { bpf_get_current_comm(meta.comm.as_mut_ptr() as *mut core::ffi::c_void, 16) };
    }
    meta.identity_hits = meta.identity_hits.saturating_add(1);
    let _ = META.insert(&cgroup_id, &meta, 0);

    let _ = EVENTS.output(
        &Delta {
            cookie,
            cgroup_id,
            pid: meta.pid,
            tid: meta.tid,
            delta_bytes: 0,
            direction: 0,
            source: SRC_SOCK_OPS,
            comm: meta.comm,
        },
        0,
    );
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
