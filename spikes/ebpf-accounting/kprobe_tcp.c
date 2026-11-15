// SPDX-License-Identifier: GPL-2.0
// eBPF accounting spike — kprobe-based per-app byte attribution
//
// Build:
//   clang -target bpf -O2 -g -Wall -D__TARGET_ARCH_x86 -I/usr/include -c kprobe_tcp.c -o kprobe_tcp.o
//
// Load (as root):
//   bpftool prog load kprobe_tcp.o /sys/fs/bpf/kprobe_tcp type kprobe name kprobe_tcp_sendmsg
//   bpftool prog load kprobe_tcp.o /sys/fs/bpf/kprobe_tcp type kprobe name kprobe_tcp_recvmsg
//
// Attach (as root):
//   bpftool prog attach id <prog_id> /sys/kernel/debug/tracing/kprobe_events type kprobe
//
// Read maps (as root):
//   bpftool map dump name byte_counts_map

#include <linux/bpf.h>
#include <stdint.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>

/* Socket cookie -> (pid_tgid, bytes_up, bytes_down) identity map */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 65536);
	__uint(key_size, sizeof(uint64_t)); /* socket cookie */
	__uint(value_size, sizeof(struct {
		uint32_t pid_tgid;
		uint64_t bytes_up;
		uint64_t bytes_down;
	}));
} sock_stats_map SEC(".maps");

/* Per-PID byte counters (fallback if socket cookie not available) */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 65536);
	__uint(key_size, sizeof(uint32_t)); /* pid_tgid */
	__uint(value_size, sizeof(struct {
		uint64_t bytes_up;
		uint64_t bytes_down;
	}));
} pid_stats_map SEC(".maps");

/* Ring buffer for events */
struct {
	__uint(type, BPF_MAP_TYPE_RINGBUF);
	__uint(max_entries, 1 << 20);
} events_rb SEC(".maps");

/* kprobe on tcp_sendmsg - count TX bytes */
SEC("kprobe/tcp_sendmsg")
int kprobe_tcp_sendmsg(struct pt_regs *ctx)
{
	void *sk = (void *)PT_REGS_PARM1(ctx);
	if (!sk)
		return 0;

	uint64_t cookie = BPF_CORE_READ(sk, sk_cookie);
	uint32_t pid_tgid = bpf_get_current_pid_tgid();

	// Try to read bytes from sk_wmem_queued or similar
	// For now, just record the socket->pid mapping
	struct {
		uint32_t pid_tgid;
		uint64_t bytes_up;
		uint64_t bytes_down;
	} *stats = bpf_map_lookup_elem(&sock_stats_map, &cookie);
	if (stats) {
		// Already have this socket, just ensure pid is set
		if (stats->pid_tgid == 0)
			stats->pid_tgid = pid_tgid;
	} else {
		struct {
			uint32_t pid_tgid;
			uint64_t bytes_up;
			uint64_t bytes_down;
		} new_stats = {pid_tgid, 0, 0};
		bpf_map_update_elem(&sock_stats_map, &cookie, &new_stats, BPF_ANY);
	}

	return 0;
}

/* kprobe on tcp_recvmsg - count RX bytes */
SEC("kprobe/tcp_recvmsg")
int kprobe_tcp_recvmsg(struct pt_regs *ctx)
{
	void *sk = (void *)PT_REGS_PARM1(ctx);
	if (!sk)
		return 0;

	uint64_t cookie = BPF_CORE_READ(sk, sk_cookie);
	uint32_t pid_tgid = bpf_get_current_pid_tgid();

	struct {
		uint32_t pid_tgid;
		uint64_t bytes_up;
		uint64_t bytes_down;
	} *stats = bpf_map_lookup_elem(&sock_stats_map, &cookie);
	if (stats) {
		if (stats->pid_tgid == 0)
			stats->pid_tgid = pid_tgid;
	} else {
		struct {
			uint32_t pid_tgid;
			uint64_t bytes_up;
			uint64_t bytes_down;
		} new_stats = {pid_tgid, 0, 0};
		bpf_map_update_elem(&sock_stats_map, &cookie, &new_stats, BPF_ANY);
	}

	return 0;
}

char _license[] SEC("license") = "GPL";
int _version SEC("version") = 1;
