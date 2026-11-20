# Phase B spike — per-app byte accounting (Aya)

**Owner-run, real root, real PC. Time box: 25 hours. Throwaway code.**

This directory is deliberately **outside the product Cargo workspace**. It is
not imported by `netwatchd`, adds no runtime dependency to the product, and
changes no product behaviour. Its only job is to replace the `[HYPOTHESIS]`
kernel claims in the docs with measurements, then let the owner accept (or
reject) `docs/spike-ebpf-report.md`. Contract:
`docs/LIVE-THROUGHPUT-PLAN.md` §5.

The earlier C/libbpf spike (`../ebpf-accounting/`) produced **no measurements**:
attach returned `-EPERM` because `/sys/fs/cgroup` was bpffs and `CAP_BPF` was
only in the bounding set. It also mis-tested cgroup_skb attach types 0–15.
This rebuild uses the mandated Aya stack and a correct attach method.

---

## 1. Toolchain (record exact versions in the report)

Verified in this repo on 2026-11-19: `aya 0.16.0`, `aya-ebpf 0.3.0`,
`aya-build 0.2.1`, Rust **nightly 1.102.0**, `bpf-linker v0.13.0`,
kernel `7.3.4-1-cachyos`, x86_64 with BTF at `/sys/kernel/btf/vmlinux`.

On your PC (Arch/CachyOS):

```bash
rustup toolchain install nightly
rustup component add --toolchain nightly rust-src llvm-tools-preview
cargo install bpf-linker
sudo pacman -S iperf3 bpftool    # workloads + inspection
```

You also need **real root** (not just `sudo` with a restricted capability
set) and a writable **cgroup v2** mount.

## 2. Build

```bash
cd spikes/ebpf-aya
cargo build -p netwatch-spike-userspace
```

The build script cross-compiles the sibling `ebpf` crate for
`bpfel-unknown-none` and embeds the object into the host binary. Build **only
the userspace package** — a bare `cargo build` also tries to link the no_std
program for the host and fails by design. `AYA_BUILD_SKIP=1` skips the nested
eBPF build if you only need to recompile the host CLI.

## 3. Diagnose first (no root needed)

```bash
./target/debug/netwatch-spike-userspace --env-only
```

This prints kernel, BTF, cgroup2 mounts, the `cgroup.controllers` marker at
your target path, and **effective vs bounding** `CAP_BPF`, `CAP_PERFMON`,
`CAP_NET_ADMIN`, `CAP_SYS_ADMIN`. If `CAP_BPF effective=false`, attach will
fail with `-EPERM` — fix the environment (or use real root) before continuing.
Record this block verbatim in the report: it is the answer to Q4/Q6.

## 4. Run the measurement

```bash
sudo ./target/debug/netwatch-spike-userspace \
  --cgroup /sys/fs/cgroup \
  --duration 60 \
  --interval 1
```

Keep links alive while it runs. Start the workload in another terminal while
the counter is at `[+00s]`. Every second it prints:

- attributed rx/tx bytes (from cgroup_skb) vs interface-counter rx/tx deltas
  (from `/sys/class/net`, `lo` excluded) and the percentage — **Q1/Q2**
- number of cookies and how many have a pid/cgroup identity — **Q5**
- ring events processed / malformed / drained — **Q8**

At the end it prints a Markdown table per socket cookie ready to paste into
the report. Sample CPU and RSS externally in a third terminal:

```bash
pidstat -p $(pidof netwatch-spike-userspace) 1
```

which answers **Q3** (idle, 50 connections, 500 connections, full-rate
download). To measure **Design 1 alone**, add `--no-kprobes` and re-run;
compare the two tables to decide Design 1 vs Design 3 (hybrid).

If `--cgroup /sys/fs/cgroup` fails, find the real cgroup2 mount from step 3
(or `stat -fc %T /sys/fs/cgroup`) and pass that path. Do **not** attach to a
bpffs mount.

> **Known finding (first owner run, 2026-11-20):** cgroup_skb ingress/egress
> attach fine, but the kernel verifier **rejects `bpf_get_socket_cookie` in a
> kprobe program** (`program of this type cannot use helper #46`). The runner
> now detects this and **continues with Design 1 byte counters only** — you
> will see `kprobe_identity load REJECTED by verifier` and then the normal
> per-second lines. That is expected; record it for Q1/Q5. For the Design 1 vs
> hybrid comparison use `--no-kprobes` as the baseline (both runs then have
> identity = 0 until a `sock_ops`-based identity probe is implemented).
> See `docs/spike-ebpf-report.md` "First owner run".

> **Q5 IDENTITY WORKS (sixth owner run, 2026-11-20):** 12/37 cookies were
> identified with real pid/cgroup_id. sock_ops attach with `Single` succeeded
> (AllowMultiple returns EINVAL code 22 at the root cgroup); the
> egress-current capture also works without any extra attach. The runner now
> tries `AllowMultiple → AllowOverride → Single` so the least intrusive mode
> wins. What is **not** yet verified: whether Motrix and its `aria2c` child
> map to the same cgroup — run once more with Motrix active and inspect the
> final table's pid/cgroup_id columns. `docs/spike-ebpf-report.md` §Sixth run.

> **Motrix verified (fifth owner run, 2026-11-20):** with Motrix downloading
> alongside the built-in 100 MB curl, cgroup_skb counted every large cookie
> (8–117 MB each, ~715 MB of Motrix traffic) at 96.5% of interface counters,
> 0 bad events, 0.0% CPU / 4,412 KiB RSS. The "820% of true bytes" line is
> expected — the built-in workload reports only curl's own 100 MB, while
> Motrix traffic was counted in the same window. **sock_ops link attach
> failed** in that run (load succeeded), so identity is still 0/243. The next
> build prints the underlying errno, retries with `Single` mode, and also
> tries `bpf_get_current_pid_tgid` in cgroup_skb egress as a zero-attach
> identity experiment. Re-run the same one-terminal command to see whether
> `pid`/`cgroup_id` become nonzero.

> **Q5 follow-up (added 2026-11-20):** a `sock_ops` identity probe is now
> included and enabled by default. It runs at connection establishment, where
> `bpf_get_socket_cookie` and `bpf_get_current_pid_tgid` are both permitted
> (unlike kprobes), and attaches with `AllowMultiple` so it cannot replace an
> existing system program. Run the same one-terminal command again and check
> whether the final table shows nonzero `pid`/`cgroup_id` and `identity_hits`
> — that answers Q5 and decides whether Phase C can show per-app rows.
> `--no-sock-ops` reproduces the byte-only baseline.

> **VALID RESULT (fourth owner run, 2026-11-20):** with a real
> `curl` workload and the accept-verdict fix, cgroup_skb attributed **95.9%
> of download bytes** vs interface counters (4.1% gap, inside the 5% go
> threshold); the 1 Hz ratio stayed at 94.3–95.7% during steady download;
> CPU/RSS were 0.0% / 4,116 KiB. Identity was 0/91 (kprobe rejected), so
> per-app rows need a `sock_ops` identity probe in Phase C. Full analysis in
> `docs/spike-ebpf-report.md` §Fourth owner run. **Byte accounting: GO;
> identity: open.**

> The runner now also has `--workload`: it spawns curl to download
> `https://proof.ovh.net/files/100Mb.dat` for the collection window and
> reports curl's exact byte count as the accuracy truth. It self-samples
> CPU/RSS from `/proc`, so you need only **one terminal**:
> `sudo ./target/debug/netwatch-spike-userspace --workload --duration 180`.

> **CRITICAL (third owner run, 2026-11-20):** the first eBPF build returned
> `0` from the cgroup_skb programs, which makes the kernel **drop every
> packet** — while attached, all other downloads and web access stop. Fixed:
> the programs now return `1` (accept); verified `r0 = 0x1` before `exit` in
> the compiled object. **All byte numbers from the earlier attached runs are
> invalid** and were discarded. If you ever see traffic stop during the
> spike, Ctrl-C immediately — a monitor-only program must never drop packets.

> **Second owner run (2026-11-20):** with no workload running, interface
> deltas were ~0 and the accuracy percentage was meaningless. The runner now
> (a) compares per-interval deltas on both sides, (b) compares cumulative vs
> cumulative in the final summary, and (c) prints `WARNING: no interface
> traffic` when idle. **You must have a workload running in a second terminal**
> for the 60 s — an idle run cannot answer Q1/Q2. (`pidstat` now uses the
> real PID instead of a `pidof` name that matched nothing.)

## 5. Workloads — Motrix first

Run each separately; record interface delta, attributed total, per-app split,
and the unattributed remainder.

| Workload | What it proves |
|---|---|
| Motrix HTTP download | TCP attribution; does the `aria2c` child land in the same cgroup/app scope? |
| Motrix torrent download | UDP/uTP/DHT counted — `sock_diag` would miss it |
| Motrix from desktop launcher, then from a terminal | Does cgroup identity change with launch method? |
| `curl` large file; `iperf3` TCP and UDP | Accuracy against a known rate (go/no-go input) |
| 5,000 short `curl` runs | Short-lived processes and map churn |
| Python script making requests | Scripts not collapsing into one `python3` row |

For each, also note which kernel symbols the kprobe attached to (the runner
prints per-symbol success/failure) — a failed `udp_recvmsg` attach explains a
UDP-attribution gap.

## 6. Questions to answer (copy into the report)

| # | Question | Evidence to paste |
|---|---|---|
| Q1 | Per-app bytes vs interface counters — share attributed, composition of gap? | interval lines + final table |
| Q2 | Is the 1 s per-app rate stable enough (jitter, lag)? | 60 interval lines under a steady `iperf3` run |
| Q3 | CPU/RSS at idle, 50 and 500 connections, full-rate download? | `pidstat` transcripts |
| Q4 | Which capabilities are actually required to load/attach/run? | step 3 block + attach errors |
| Q5 | Does cgroup identity group Motrix + `aria2c`; can a name come from `.desktop`? | per-cookie pid/cgroup table + `systemd-cgls` |
| Q6 | Does Secure Boot / kernel lockdown block anything? | repeat on Ubuntu LTS and Fedora VMs with Secure Boot on |

## 7. Go / no-go (owner decides)

- **Go** if `curl`/`iperf3` attributed bytes are within ~5% of the true
  traffic, attributed + unattributed reconciles with interface counters
  within ~2%, and CPU/RSS stay inside ≤3% / ≤150 MB.
- **Re-scope** if UDP or short-lived processes cannot be attributed reliably.
  Per owner decision **L-OQ3**, this does **not** block Phase C: ship per-app
  speeds with an explicit **"TCP only"** label plus an Unattributed remainder,
  provided the sum check still holds.
- Write every result into `docs/spike-ebpf-report.md`, re-tag each claim
  `[VERIFIED: spike-ebpf-report §n]` or `[REFUTED]`, and **do not start Phase C
  before the owner accepts the report.**

A second kernel (Ubuntu LTS or Fedora VM, real VM not container) is required
for Q6 before sign-off.

## 8. Stop condition

Stop at the 25-hour box and report whatever exists. A negative result is a
valid result — it is what decides the "TCP only" label. The earlier failed
spike's environment notes are in `docs/spike-ebpf-report.md`.
