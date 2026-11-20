//! Phase B spike — eBPF programs (Aya), throwaway measurement code.
//!
//! Design 1: cgroup_skb ingress/egress count L3 bytes per socket cookie.
//! Loopback traffic is tagged and reported separately, never mixed into the
//! interface-counter comparison.
//! Identity: captured at socket establishment by sock_ops (active/passive
//! callbacks), where the socket owner is `current`. The earlier kprobe
//! approach was verifier-rejected (bpf_get_socket_cookie not allowed there)
//! and has been removed.
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
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_socket_cookie,
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
/// 0 and accepts it on any nonzero value. Returning 0 here dropped ALL
/// traffic while the programs were attached (observed 2026-11-20).
const CGROUP_SKB_ACCEPT: i32 = 1;

/// Configuration written by userspace before attach.
/// index 0 = the ifindex of `lo`, so loopback can be filtered by number
/// rather than by name (the correct way to compare with skb->ifindex).
#[map]
static CONFIG: Array<u32> = Array::with_max_entries(1, 0);

/// Identity attached to a socket cookie, captured at connect/accept time.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Meta {
    /// TGID (the user-visible PID) from the high half of bpf_get_current_pid_tgid.
    pub pid: u32,
    /// Thread ID from the low half.
    pub tid: u32,
    pub cgroup_id: u64,
    /// Process name from bpf_get_current_comm (may not be NUL-terminated).
    pub comm: [u8; 16],
    pub ingress_hits: u32,
    pub egress_hits: u32,
    pub identity_hits: u32,
}

/// Cumulative counters per cookie (useful when the ring buffer drops events).
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
    pub cookie: u64,
    pub pid: u32,
    pub tid: u32,
    pub cgroup_id: u64,
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
        cgroup_id: 0,
        comm: [0u8; 16],
        ingress_hits: 0,
        egress_hits: 0,
        identity_hits: 0,
    }
}

fn read_meta(cookie: u64) -> Meta {
    match META.get_ptr_mut(&cookie) {
        Some(ptr) => unsafe { *ptr },
        None => empty_meta(),
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
    // Read ifindex / ingress_ifindex straight from the __sk_buff context.
    // These are context fields, safe for the verifier to read in cgroup_skb.
    let skb = unsafe { &*(ctx.as_ptr() as *const __sk_buff) };
    let ifindex = skb.ifindex;
    let ingress_ifindex = skb.ingress_ifindex;

    // Loopback is injected from userspace so we never hardcode "ifindex 1".
    let lo_ifindex = CONFIG.get(0).copied().unwrap_or(0);
    let is_loopback = lo_ifindex != 0 && (ifindex == lo_ifindex || ingress_ifindex == lo_ifindex);

    let cookie = unsafe { bpf_get_socket_cookie(ctx.as_ptr()) };
    let len = u64::from(ctx.len());
    let meta = read_meta(cookie);
    let source = if is_loopback {
        SRC_LOOPBACK
    } else {
        SRC_CGROUP_SKB
    };

    if !is_loopback {
        let mut counts = match COUNTS.get_ptr_mut(&cookie) {
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
        let _ = COUNTS.insert(&cookie, &counts, 0);
    }

    let mut meta = meta;
    if direction == 0 {
        meta.ingress_hits = meta.ingress_hits.saturating_add(1);
    } else {
        meta.egress_hits = meta.egress_hits.saturating_add(1);
    }
    let _ = META.insert(&cookie, &meta, 0);

    // Emit every packet's delta; userspace separates loopback by `source`.
    // This is high-volume but bounded by the 256 KiB ring buffer.
    let _ = EVENTS.output(
        &Delta {
            cookie,
            pid: meta.pid,
            tid: meta.tid,
            cgroup_id: meta.cgroup_id,
            delta_bytes: len,
            direction,
            source,
            comm: meta.comm,
        },
        0,
    );
    // Observe-only: return nonzero so the kernel ACCEPTS the packet. 0 drops.
    CGROUP_SKB_ACCEPT
}

/// Capture cookie -> (pid, tid, cgroup_id, comm) at connection establishment.
///
/// sock_ops runs in the process context of connect()/accept(), so
/// `bpf_get_current_pid_tgid` names the actual socket owner. This replaces
/// the old kprobe (verifier-rejected) and the unreliable egress-current guess.
/// It never calls set_reply — observe-only.
#[sock_ops]
pub fn identity(ctx: SockOpsContext) -> u32 {
    let op = ctx.op();
    if op != BPF_SOCK_OPS_ACTIVE_ESTABLISHED_CB && op != BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB {
        return 0;
    }
    let cookie = unsafe { bpf_get_socket_cookie(ctx.as_ptr()) };
    if cookie == 0 {
        return 0;
    }
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let cgroup_id = unsafe { bpf_get_current_cgroup_id() };

    let mut meta = empty_meta();
    meta.pid = (pid_tgid >> 32) as u32;
    meta.tid = pid_tgid as u32;
    meta.cgroup_id = cgroup_id;
    let _ = unsafe { bpf_get_current_comm(meta.comm.as_mut_ptr() as *mut core::ffi::c_void, 16) };
    meta.identity_hits = 1;
    let _ = META.insert(&cookie, &meta, 0);

    let _ = EVENTS.output(
        &Delta {
            cookie,
            pid: meta.pid,
            tid: meta.tid,
            cgroup_id: meta.cgroup_id,
            delta_bytes: 0,
            direction: 0,
            source: SRC_SOCK_OPS,
            comm: meta.comm,
        },
        0,
    );
    0 // return value is ignored for ESTABLISHED_CB; never modify socket state
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
