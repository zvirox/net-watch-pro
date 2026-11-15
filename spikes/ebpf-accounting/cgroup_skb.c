// SPDX-License-Identifier: GPL-2.0
// eBPF accounting spike — per-app byte attribution measurement (Part 3).
//
// Build:
//   clang -target bpf -O2 -g -Wall -D__TARGET_ARCH_x86 -I/usr/include -c cgroup_skb.c -o cgroup_skb.o
//
// Load (as root):
//   bpftool prog load cgroup_skb.o /sys/fs/bpf/cgroup_skb type cgroup_skb
//
// Read maps (as root):
//   bpftool map dump name byte_counts_map

#include <linux/bpf.h>
#include <stdint.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>

/* Per-5tuple byte counters keyed by (remote_ip, remote_port, local_port, family) */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 65536);
	__uint(key_size, 16); /* 4 x uint32_t */
	__uint(value_size, 16); /* 2 x uint64_t */
} byte_counts_map SEC(".maps");

/* Per-interface byte counters (for interface comparison) */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 4096);
	__uint(key_size, 4); /* uint32_t ifindex */
	__uint(value_size, 16); /* 2 x uint64_t */
} iface_counts_map SEC(".maps");

static __always_inline void count_bytes(struct __sk_buff *skb, uint32_t bytes, int is_egress)
{
	uint32_t remote_ip = BPF_CORE_READ(skb, remote_ip4);
	uint32_t remote_port = BPF_CORE_READ(skb, remote_port);
	uint32_t local_port = BPF_CORE_READ(skb, local_port);
	uint32_t family = BPF_CORE_READ(skb, family);

	/* Key as raw bytes: remote_ip(4) + remote_port(4) + local_port(4) + family(4) */
	uint8_t key[16] = {0};
	__builtin_memcpy(key, &remote_ip, 4);
	__builtin_memcpy(key + 4, &remote_port, 4);
	__builtin_memcpy(key + 8, &local_port, 4);
	__builtin_memcpy(key + 12, &family, 4);

	struct {
		uint64_t bytes_up;
		uint64_t bytes_down;
	} *cnts = bpf_map_lookup_elem(&byte_counts_map, key);
	if (cnts) {
		if (is_egress)
			__sync_fetch_and_add(&cnts->bytes_up, bytes);
		else
			__sync_fetch_and_add(&cnts->bytes_down, bytes);
	} else {
		struct {
			uint64_t bytes_up;
			uint64_t bytes_down;
		} zero = {is_egress ? bytes : 0, is_egress ? 0 : bytes};
		bpf_map_update_elem(&byte_counts_map, key, &zero, BPF_ANY);
	}
}

/* =========================================================================
 * Design 1: cgroup_skb ingress — count RX bytes per 5tuple
 * ========================================================================= */

SEC("cgroup_skb/ingress")
int cgroup_skb_ingress(struct __sk_buff *skb)
{
	uint32_t len = BPF_CORE_READ(skb, len);
	count_bytes(skb, len, 0);
	return 0;
}

/* =========================================================================
 * Design 1: cgroup_skb egress — count TX bytes per 5tuple
 * ========================================================================= */

SEC("cgroup_skb/egress")
int cgroup_skb_egress(struct __sk_buff *skb)
{
	uint32_t len = BPF_CORE_READ(skb, len);
	count_bytes(skb, len, 1);
	return 0;
}

char _license[] SEC("license") = "GPL";
int _version SEC("version") = 1;
