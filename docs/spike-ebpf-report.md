# eBPF Accounting Spike Report

**Date:** 2026-11-15  
**Environment:** Arch/CachyOS, kernel 7.3.4-1-cachyos, cgroup v2, WebKit2GTK 2.54.2  
**Toolchain:** Rust nightly 1.102.0, clang 23.1.5, bpf-linker v0.13.0, libbpf 1.x  
**Status:** Phase B (Aya rebuild) pending owner run — see below. The C/libbpf
attempt documented under "Historical" is retained for diagnosis only.

---

## Phase B (Aya rebuild) — PENDING OWNER RUN (2026-11-19)

The throwaway Aya spike is at `spikes/ebpf-aya/`; follow its README on your
real PC with real root. Paste the runner's final table here, then re-tag the
docs' kernel claims `[VERIFIED: spike-ebpf-report §n]` or `[REFUTED]`. Do not
start Phase C until the owner accepts this report
(`docs/LIVE-THROUGHPUT-PLAN.md` §5/§6).

### First owner run — 2026-11-20 (partial, real root on CachyOS)

Environment block: `CAP_{BPF,PERFMON,NET_ADMIN,SYS_ADMIN} effective=true`,
BTF present, `cgroup.controllers` present at `/sys/fs/cgroup` (so **the old
`-EPERM` blocker is resolved** — the earlier failure was the restricted
environment, now confirmed `[REFUTED]` for a real root shell).

**Design 1 (cgroup_skb counters) attaches successfully:**
```
attaching cgroup_skb ingress/egress to "/sys/fs/cgroup"
  ingress: attached
  egress: attached
```
→ **[VERIFIED: spike-ebpf-report §First owner run]** cgroup_skb ingress and
egress attach to cgroup v2 with the bounded capability set on this kernel.

**Design 2 as first written is REFUTED by the verifier:**
```
Error: loading kprobe_identity — Invalid argument (os error 22)
  program of this type cannot use helper bpf_get_socket_cookie#46
  Verification failed: Policy: Operation is not allowed
```
→ **[REFUTED: spike-ebpf-report §First owner run]** calling
`bpf_get_socket_cookie` from a kprobe is not permitted (kprobe program type
cannot use that helper). Cookie→identity capture must move to a program type
that allows it — the documented candidate is `sock_ops` at
`ACTIVE_ESTABLISHED_CB`/`PASSIVE_ESTABLISHED_CB`, which runs in process context
and permits both `bpf_get_socket_cookie` and `bpf_get_current_pid_tgid`.
This is a finding for Phase C design, **not** a blocker: the runner now
degrades gracefully and continues with Design 1 byte counters, so the accuracy
workload (Q1/Q2) can still be measured.

### Second owner run — 2026-11-20 (60 s, still no workload)

The runner completed the full 60 s. Findings:

- **cgroup_skb byte counting works**: 607 ring events, 0 bad, 95 distinct
  cookie/socket keys, tx attributed 127,408 B cumulatively. No drops detected.
  → **[VERIFIED: spike-ebpf-report §Second owner run]** cgroup_skb ingress and
  egress count bytes and flush to userspace with the bounded capability set.
- **No workload was running.** Interface deltas were ~0 (only background
  blips of 70–200 B), so this run **cannot answer Q1/Q2**. Attributed rx was
  2,100 B over 60 s — consistent with an idle PC, not a known rate. The runner
  now prints an explicit `WARNING: no interface traffic` in this case.
- **Q5 unmeasured**: identity_hits = 0 for every cookie (kprobe rejected).
- **Q8 partial**: 95 cookies in 60 s shows high short-lived socket churn;
  the 8,192-entry LRU map did not fill, 0 dropped events observed in 607.
- **Reporting bug found and fixed after this run**: the live line compared
  *cumulative* attributed bytes against a *per-interval* interface delta,
  producing meaningless percentages (>1000%). Q1/Q2 numbers from this
  specific run are therefore **discarded**, not reported. The runner now
  compares per-interval to per-interval and cumulative-to-cumulative.

**Next step:** re-run with a real workload active (curl/iperf3/Motrix) in a
second terminal, plus `pidstat` for Q3. Record those numbers here.

### Third owner run — 2026-11-20 (180 s, CRITICAL BUG FOUND)

The owner reported that **while the spike was attached, all other downloads
and web access stopped working**. Cause found and fixed:

- The cgroup_skb programs returned `0`. For `BPF_PROG_TYPE_CGROUP_SKB` the
  kernel **drops the packet when the program returns 0**; observe-only programs
  must return a nonzero verdict (1). Because the spike attaches to the root
  cgroup, this dropped traffic for the whole system — not monitor-only.
  → **[VERIFIED: spike-ebpf-report §Third owner run]** returning 0 from
  cgroup_skb drops packets under kernel 7.3.4-cachyos. **All previous byte
  counts from attached runs are INVALID** and discarded (the programs were
  dropping most traffic, so attributed bytes are meaningless).
- **Fix:** return `CGROUP_SKB_ACCEPT = 1`. Verified in the compiled BPF object:
  `r0 = 0x1` before `exit` in `cgroup_skb/egress`.
- **Product implication for Phase C:** any cgroup_skb-based collector must
  return the accept verdict, and the spike/first integration must explicitly
  test that traffic still flows while attached. This is a hard safety gate.

The third run's numbers are discarded for the same reason. The next run is
the first one that can yield valid Q1/Q2 data.

| # | Question | Measurement / evidence | Status |
|---|---|---|---|
| Q1 | Per-app bytes vs interface counters; composition of the gap | | not measured |
| Q2 | 1 s per-app rate stability (jitter, lag) | | not measured |
| Q3 | CPU/RSS at idle, 50/500 connections, full-rate download | | not measured |
| Q4 | Capabilities actually required to load/attach/run | | not measured |
| Q5 | cgroup identity for Motrix + `aria2c`; `.desktop` name | | not measured |
| Q6 | Secure Boot / lockdown on Ubuntu LTS + Fedora VMs | | not measured |

**Go/no-go inputs:** attributed vs true ≤ 5% (curl/iperf3); attributed +
unattributed vs interface ≤ 2%; CPU ≤ 3%; RSS ≤ 150 MB. Per owner L-OQ3, a
UDP/short-lived gap means "TCP only" labelling in Phase C, not a block.

---

## Historical: earlier C/libbpf attempt (no measurements)

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
