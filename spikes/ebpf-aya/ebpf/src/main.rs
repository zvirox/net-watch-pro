//! Phase B spike — eBPF programs (Aya), throwaway measurement code.
//!
//! Design 1: cgroup_skb ingress/egress count L3 bytes per socket cookie.
//! Design 2: a generic kprobe captures cookie -> (pid, cgroup) identity on
//!           tcp/udp send/recv. Attach it to several symbols from userspace
//!           to build Design 3 (hybrid): Design 1 for bytes, Design 2 for
//!           identity where the cookie map has no entry.
//!
//! Nothing here is wired into netwatchd. The owner runs this on real root and
//! records measurements in docs/spike-ebpf-report.md (LIVE-THROUGHPUT-PLAN §5).

#![no_std]
#![no_main]

use core::ffi::c_void;

use aya_ebpf::{
    helpers::generated::{
        bpf_get_current_cgroup_id, bpf_get_current_pid_tgid, bpf_get_socket_cookie,
    },
    macros::{cgroup_skb, kprobe, map},
    maps::{HashMap, RingBuf},
    programs::{ProbeContext, SkBuffContext},
    EbpfContext as _,
};

const MAX_ENTRIES: u32 = 8192;
const RING_BYTES: u32 = 256 * 1024;
const SRC_CGROUP_SKB: u32 = 1;
const SRC_KPROBE: u32 = 2;

/// Identity attached to a socket cookie. `identity_hits` shows whether the
/// Design 2 helper ever ran for that socket — the measure of identity coverage.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Meta {
    pub pid: u32,
    pub cgroup_id: u64,
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
    pub cgroup_id: u64,
    pub delta_bytes: u64,
    pub direction: u32, // 0 = rx/ingress, 1 = tx/egress
    pub source: u32,    // SRC_CGROUP_SKB or SRC_KPROBE
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
        cgroup_id: 0,
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
    let cookie = unsafe { bpf_get_socket_cookie(ctx.as_ptr()) };
    let len = u64::from(ctx.len());

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

    let mut meta = read_meta(cookie);
    if direction == 0 {
        meta.ingress_hits = meta.ingress_hits.saturating_add(1);
    } else {
        meta.egress_hits = meta.egress_hits.saturating_add(1);
    }
    let _ = META.insert(&cookie, &meta, 0);

    let _ = EVENTS.output(
        &Delta {
            cookie,
            pid: meta.pid,
            cgroup_id: meta.cgroup_id,
            delta_bytes: len,
            direction,
            source: SRC_CGROUP_SKB,
        },
        0,
    );
    0 // CGROUP_SKB_OK — observe only, never drop traffic
}

/// Generic kprobe: userspace attaches it to tcp_sendmsg, tcp_recvmsg,
/// udp_sendmsg, udp_recvmsg. arg0 is `struct sock *` for all four.
#[kprobe]
pub fn kprobe_identity(ctx: ProbeContext) -> u32 {
    let sk: u64 = match ctx.arg(0) {
        Some(value) => value,
        None => return 0,
    };
    let cookie = unsafe { bpf_get_socket_cookie(sk as *mut c_void) };
    if cookie == 0 {
        return 0;
    }
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let cgroup = unsafe { bpf_get_current_cgroup_id() };

    let mut meta = read_meta(cookie);
    meta.pid = (pid_tgid >> 32) as u32;
    meta.cgroup_id = cgroup;
    meta.identity_hits = meta.identity_hits.saturating_add(1);
    let _ = META.insert(&cookie, &meta, 0);

    let _ = EVENTS.output(
        &Delta {
            cookie,
            pid: meta.pid,
            cgroup_id: meta.cgroup_id,
            delta_bytes: 0,
            direction: 0,
            source: SRC_KPROBE,
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
