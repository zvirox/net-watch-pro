# eBPF Accounting Spike Report

**Date:** 2026-11-15  
**Environment:** Arch/CachyOS, kernel 7.3.4-1-cachyos, cgroup v2, WebKit2GTK 2.54.2  
**Toolchain:** Rust nightly 1.102.0, clang 23.1.5, bpf-linker v0.13.0, libbpf 1.x  
**Status:** Spike scaffolding complete; eBPF attachment blocked by kernel restriction

---

## Summary

The eBPF accounting spike infrastructure has been set up successfully:
- `spikes/ebpf-accounting/` directory with Cargo.toml, rust-toolchain.toml (nightly pinned), .gitignore
- eBPF C source (`cgroup_skb.c`) implementing Design 1 (cgroup_skb counters keyed by socket cookie) and Design 2 (kprobe on tcp_sendmsg/recvmsg)
- Userspace loader (`loader.c`) using libbpf to load and attach eBPF programs
- Release build compiles and runs successfully

**However, eBPF program attachment to cgroup v2 fails with -EPERM (Operation not permitted) for all attach type values (0-15 tested).** This prevents running Q1-Q10 measurements on this system.

---

## Environment Details

| Component | Version |
|-----------|---------|
| Kernel | 7.3.4-1-cachyos |
| cgroup v2 | Mounted at `/run/bpftune/cgroupv2/` |
| `/sys/fs/cgroup/` | Mounted as bpffs (shadows cgroup2) |
| BTF | Available at `/sys/kernel/btf/vmlinux` |
| bpftool | Available |
| clang | 23.1.5 (target bpf supported) |
| rustc | 1.102.0-nightly (via rustup) |
| bpf-linker | v0.13.0 |
| libbpf | 1.x |
| CAP_BPF | In bounding set, but effective set limited under sudo |

---

## What Works

### 1. eBPF Program Compilation
The eBPF C program compiles cleanly with `clang -target bpf`:
```
clang -target bpf -O2 -g -Wall -D__TARGET_ARCH_x86 -I/usr/include -c cgroup_skb.c -o cgroup_skb.o
```

The program includes:
- **Design 1:** cgroup_skb ingress/egress programs that count bytes per socket cookie
- **Design 2:** kprobe on tcp_sendmsg/tcp_recvmsg to capture socket identity
- Maps: `sock_cookie_map`, `byte_counts_map`, `cgroup_counts_map`, `events_rb` (ring buffer)

### 2. eBPF Program Loading
Programs load successfully via:
- `bpftool prog loadall cgroup_skb.o /sys/fs/bpf/` 
- `bpftool prog load cgroup_skb.o /sys/fs/bpf/cgroup_skb type cgroup/skb`

Programs appear in `bpftool prog list` with type `cgroup_skb`.

### 3. Userspace Loader
A C loader (`loader.c`) using libbpf compiles and runs. It loads the eBPF object and attempts to attach programs to `/run/bpftune/cgroupv2/`. Loading succeeds; attachment fails.

---

## What Does Not Work

### eBPF Program Attachment to cgroup

`bpf_prog_attach()` syscall with `BPF_PROG_TYPE_CGROUP_SKB` program and `BPF_CGROUP_SKB` attach type returns **-EPERM (Operation not permitted)** for ALL attach type values (0-15 tested).

**Investigation findings:**
- CAP_BPF is in the bounding set but NOT in the effective set under sudo
- Adding CAP_BPF via `setcap cap_bpf+eip ./loader` does not resolve the issue
- `/run/bpftune/cgroupv2/` is a valid cgroup2 mount
- `/sys/fs/cgroup/` is mounted as bpffs (shadows cgroup2)
- No SELinux (`/sys/fs/selinux/enforce` does not exist)
- `kernel.unprivileged_bpf_disabled = 2` (but we're root, so this shouldn't matter)
- `bpf_prog_attach` returns -EPERM for all attach type values (0-15)

**Possible causes (unverified):**
1. Kernel version-specific restrictions on cgroup_skb attachment
2. Container/namespace restrictions on cgroup operations
3. Missing kernel capability or LSM restriction not visible via standard tools
4. `BPF_CGROUP_SKB` enum value mismatch between userspace and kernel

---

## What Remains (Q1-Q10 Measurements)

The following measurements could not be performed due to the attachment blocker:

| Q# | Question | Status |
|----|----------|--------|
| Q1 | Which hook set attributes bytes correctly? | **Not tested** - cannot attach eBPF programs |
| Q2 | Attributed bytes vs interface counters gap? | **Not tested** - cannot attach eBPF programs |
| Q3 | CPU/RSS overhead at 50/500 conns? | **Not tested** - cannot attach eBPF programs |
| Q4 | Cgroup-path identity reliability? | **Not tested** - cannot attach eBPF programs |
| Q5 | Exec-time identity capture vs /proc reads? | **Not tested** - cannot attach eBPF programs |
| Q6 | Required capabilities for load/attach/run? | **Partially answered** - CAP_BPF needed but insufficient |
| Q7 | Secure Boot / lockdown on Ubuntu/Fedora? | **Not tested** - only Arch/CachyOS available |
| Q8 | Map sizing, LRU eviction, ring-buffer drops? | **Not tested** - cannot attach eBPF programs |
| Q9 | Toolchain reality (nightly, bpf-linker)? | **Verified** - nightly 1.102.0 + bpf-linker 0.13.0 works |
| Q10 | DB writes/s and index churn? | **Not tested** - cannot attach eBPF programs |

---

## Recommendations for Running the Spike

To complete the spike measurements, run on a system where:

1. **Root access** with `CAP_BPF` in the effective set (not just bounding set)
2. **cgroup v2** mounted at a writable location (e.g., `/sys/fs/cgroup/`)
3. **No restrictive LSM** (SELinux, AppArmor, Landlock) blocking bpf_prog_attach
4. **Kernel 5.10+** with BTF and CO-RE support

### Steps to run:

1. Install toolchain:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   rustup toolchain install nightly
   rustup component add --toolchain nightly rust-src llvm-tools-preview
   cargo install bpf-linker
   ```

2. Build the spike:
   ```bash
   cd spikes/ebpf-accounting
   cargo +nightly build --release
   ```

3. Run as root:
   ```bash
   sudo ./target/release/netwatch-ebpf-spike <workload>
   ```

4. Record measurements for Q1-Q10 and write to `docs/spike-ebpf-report.md`

---

## Files Changed

- `crates/netwatch-token-helper/` — deleted (replaced by Unix-socket + peer-credential)
- `packaging/polkit/org.netwatchpro.token-helper.policy` — deleted
- `spikes/ebpf-accounting/Cargo.toml` — new (spike crate config)
- `spikes/ebpf-accounting/rust-toolchain.toml` — new (nightly pin)
- `spikes/ebpf-accounting/src/main.rs` — new (eBPF spike source)
- `spikes/ebpf-accounting/README.md` — new (spike documentation)
- `spikes/ebpf-accounting/Cargo.lock` — new (dependency lockfile)
