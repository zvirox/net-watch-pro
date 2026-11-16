//! eBPF programs for Net Watch Pro - per-app byte attribution.
//!
//! This is spike scaffolding. PLAN.md Phase S (eBPF spike) gates this code:
//! Q1-Q10 measurements must run on real VMs (Arch/CachyOS, Ubuntu LTS, Fedora)
//! before this crate is wired into netwatchd. On this build machine, eBPF
//! program attachment returns -EPERM, so the code below compiles but is not
//! loaded into the kernel.
//!
//! Designs (from PLAN.md Part 3.3):
//!   Design 1: cgroup_skb counters keyed by socket cookie
//!   Design 2: kprobe/fentry on tcp_sendmsg/tcp_cleanup_rbuf/tcp_recvmsg
//!   Design 3: hybrid - Design 1 for bytes, Design 2 for identity
//!
//! Maps:
//!   socket_cookie_map: u64 cookie -> (pid_tgid, cgroup_id)
//!   byte_counts_map: socket cookie -> (bytes_up, bytes_down)
//!   events_ringbuf: delta events flushed to userspace ~1s cadence

#![cfg_attr(not(feature = "ebpf-programs"), allow(dead_code))]

/// Socket cookie -> identity map key (Design 1).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CookieKey {
    pub cookie: u64,
}

/// Socket cookie -> identity map value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CookieValue {
    pub pid_tgid: u32,
    pub cgroup_id: u64,
}

/// Socket cookie -> byte counters key.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BytesKey {
    pub cookie: u64,
}

/// Socket cookie -> byte counters value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BytesValue {
    pub bytes_up: u64,
    pub bytes_down: u64,
}

/// Ring buffer event for userspace delta flush.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeltaEvent {
    pub pid_tgid: u32,
    pub cgroup_id: u64,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub timestamp_ns: u64,
}

/// Design 1: cgroup_skb ingress program - count RX bytes per socket cookie.
#[cfg(feature = "ebpf-programs")]
#[no_mangle]
pub extern "C" fn cgroup_skb_ingress(_ctx: *mut std::ffi::c_void) -> u32 {
    // Placeholder: implement socket cookie extraction and byte counting
    0
}

/// Design 1: cgroup_skb egress program - count TX bytes per socket cookie.
#[cfg(feature = "ebpf-programs")]
#[no_mangle]
pub extern "C" fn cgroup_skb_egress(_ctx: *mut std::ffi::c_void) -> u32 {
    // Placeholder: implement socket cookie lookup and byte counting
    0
}

/// Design 2: kprobe on tcp_sendmsg - capture socket cookie at connect time.
#[cfg(feature = "ebpf-programs")]
#[no_mangle]
pub extern "C" fn kprobe_tcp_sendmsg(_ctx: *mut std::ffi::c_void) -> u32 {
    // Placeholder: capture socket cookie -> pid/cgroup identity mapping
    0
}

/// Design 2: kprobe on tcp_recvmsg - capture socket cookie (fallback identity).
#[cfg(feature = "ebpf-programs")]
#[no_mangle]
pub extern "C" fn kprobe_tcp_recvmsg(_ctx: *mut std::ffi::c_void) -> u32 {
    // Placeholder: capture socket cookie -> pid/cgroup identity mapping
    0
}

#[cfg(not(feature = "ebpf-programs"))]
mod stub {
    /// Compile-time feature flag guards all eBPF program code.
    /// The crate compiles without Aya until spike results confirm the hook design.
    pub mod stub {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_key_size() {
        assert_eq!(std::mem::size_of::<CookieKey>(), 8);
    }

    #[test]
    fn cookie_value_size() {
        assert_eq!(std::mem::size_of::<CookieValue>(), 12);
    }

    #[test]
    fn bytes_key_size() {
        assert_eq!(std::mem::size_of::<BytesKey>(), 8);
    }

    #[test]
    fn bytes_value_size() {
        assert_eq!(std::mem::size_of::<BytesValue>(), 16);
    }

    #[test]
    fn delta_event_size() {
        assert_eq!(std::mem::size_of::<DeltaEvent>(), 40);
    }
}
