//! Phase B spike runner — loads the Aya programs, attaches on real root, and
//! prints measurements the owner records in docs/spike-ebpf-report.md.
//!
//! Throwaway code: outside the product workspace, never imported by netwatchd.
//! See README.md in this directory for the exact runbook.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use aya::include_bytes_aligned;
use aya::maps::RingBuf;
use aya::programs::{CgroupAttachMode, CgroupSkb, CgroupSkbAttachType, KProbe, SockOps};
use aya::Ebpf;
use clap::Parser;

const INTERFACE_ROOT: &str = "/sys/class/net";
const SRC_CGROUP_SKB: u32 = 1;
const SRC_KPROBE: u32 = 2;
const SRC_SOCK_OPS: u32 = 3;

#[derive(Parser, Debug)]
#[command(name = "netwatch-spike", about = "Phase B eBPF accounting spike (Aya)")]
struct Args {
    /// cgroup v2 directory to attach cgroup_skb programs to
    #[arg(long, default_value = "/sys/fs/cgroup")]
    cgroup: PathBuf,

    /// Seconds to collect after attach
    #[arg(long, default_value_t = 30)]
    duration: u64,

    /// Seconds between printed snapshots
    #[arg(long, default_value_t = 1)]
    interval: u64,

    /// Skip Design 2 kprobes (measure Design 1 alone)
    #[arg(long)]
    no_kprobes: bool,

    /// Skip the sock_ops identity probe (Q5 candidate). Enabled by default.
    #[arg(long)]
    no_sock_ops: bool,

    /// Generate our own known-rate download during collection, so no second
    /// terminal or external download setup is needed. Uses a ranged HTTP GET.
    #[arg(long)]
    workload: bool,

    /// Host:port for --workload's HTTP server
    #[arg(long, default_value = "proof.ovh.net:80")]
    workload_host: String,

    /// Path for --workload's HTTP GET (a large, static file)
    #[arg(long, default_value = "/files/100Mb.dat")]
    workload_path: String,

    /// Print environment diagnostics only; do not load or attach eBPF
    #[arg(long)]
    env_only: bool,
}

/// Identical layout to `Delta` in the eBPF program (`#[repr(C)]`, little-endian).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Delta {
    cookie: u64,
    pid: u32,
    cgroup_id: u64,
    delta_bytes: u64,
    direction: u32,
    source: u32,
}

impl Delta {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        const N: usize = std::mem::size_of::<Delta>();
        if bytes.len() < N {
            return None;
        }
        let u64_at = |offset: usize| -> u64 {
            u64::from_ne_bytes(bytes[offset..offset + 8].try_into().expect("8 bytes"))
        };
        let u32_at = |offset: usize| -> u32 {
            u32::from_ne_bytes(bytes[offset..offset + 4].try_into().expect("4 bytes"))
        };
        Some(Self {
            cookie: u64_at(0),
            pid: u32_at(8),
            cgroup_id: u64_at(16),
            delta_bytes: u64_at(24),
            direction: u32_at(32),
            source: u32_at(36),
        })
    }
}

#[derive(Default, Debug)]
struct AppAgg {
    pid: u32,
    cgroup_id: u64,
    rx: u64,
    tx: u64,
    identity_hits: u32,
    events: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    print_environment(&args.cgroup);
    if args.env_only {
        return Ok(());
    }

    println!("\n--- load ---");
    let bytes = include_bytes_aligned!(concat!(env!("OUT_DIR"), "/netwatch-spike-ebpf"));
    let mut bpf = Ebpf::load(bytes).context("loading eBPF object (is this real root with BTF?)")?;

    let map = bpf
        .take_map("EVENTS")
        .ok_or_else(|| anyhow!("EVENTS ring buffer map missing from object"))?;
    let mut events: RingBuf<_> = map.try_into().context("opening EVENTS ring buffer")?;

    let cgroup = File::open(&args.cgroup).with_context(|| {
        format!(
            "opening cgroup {:?} — use --cgroup to point at a cgroup2 mount",
            args.cgroup
        )
    })?;

    println!("attaching cgroup_skb ingress/egress to {:?}", args.cgroup);
    {
        let ingress: &mut CgroupSkb = bpf
            .program_mut("ingress")
            .ok_or_else(|| anyhow!("ingress program missing"))?
            .try_into()?;
        ingress.load().context("loading ingress cgroup_skb")?;
        let link = ingress.attach(
            &cgroup,
            CgroupSkbAttachType::Ingress,
            CgroupAttachMode::Single,
        );
        match link {
            Ok(_) => println!("  ingress: attached"),
            Err(error) => bail!("ingress attach failed: {error} (see environment report above)"),
        }
    }
    {
        let egress: &mut CgroupSkb = bpf
            .program_mut("egress")
            .ok_or_else(|| anyhow!("egress program missing"))?
            .try_into()?;
        egress.load().context("loading egress cgroup_skb")?;
        let link = egress.attach(
            &cgroup,
            CgroupSkbAttachType::Egress,
            CgroupAttachMode::Single,
        );
        match link {
            Ok(_) => println!("  egress: attached"),
            Err(error) => bail!("egress attach failed: {error} (see environment report above)"),
        }
    }

    let mut kprobe_links = Vec::new();
    if !args.no_kprobes {
        println!("attaching Design 2 identity kprobe (hybrid):");
        // The verifier may reject bpf_get_socket_cookie in a kprobe. That is a
        // real measurement for Q1/Q5, not a fatal error: keep the cgroup_skb
        // byte counters (Design 1) running so the accuracy workload still
        // produces data. Record the result in the report either way.
        let mut identity_available = false;
        {
            if let Some(program) = bpf.program_mut("kprobe_identity") {
                match TryInto::<&mut KProbe>::try_into(program) {
                    Ok(prog) => match prog.load() {
                        Ok(()) => identity_available = true,
                        Err(error) => println!(
                            "  kprobe_identity load REJECTED by verifier: {error}\n  \
                             -> continuing with Design 1 byte counters only; record this for Q1/Q5"
                        ),
                    },
                    Err(error) => println!("  kprobe_identity wrong program type: {error}"),
                }
            } else {
                println!("  kprobe_identity program missing from object");
            }
        }
        if identity_available {
            for symbol in ["tcp_sendmsg", "tcp_recvmsg", "udp_sendmsg", "udp_recvmsg"] {
                if let Some(program) = bpf.program_mut("kprobe_identity") {
                    if let Ok(prog) = TryInto::<&mut KProbe>::try_into(program) {
                        match prog.attach(symbol, 0) {
                            Ok(link) => {
                                println!("  {symbol}: attached");
                                kprobe_links.push(link);
                            }
                            Err(error) => println!("  {symbol}: FAILED ({error}) — record this"),
                        }
                    }
                }
            }
        }
    } else {
        println!("--no-kprobes: kprobe identity disabled");
    }

    // Q5 candidate: sock_ops captures cookie -> (pid, cgroup) at connect(),
    // where both helpers are allowed. Attach with AllowMultiple so an existing
    // system sock_ops program is not replaced (monitor-only, no set_reply).
    // Held for its Drop: the link must stay alive through collect() or the
    // program detaches. Never read explicitly — that is intentional.
    let mut _sock_ops_link = None;
    if !args.no_sock_ops {
        println!("attaching sock_ops identity probe (Q5 candidate):");
        if let Some(program) = bpf.program_mut("identity") {
            match TryInto::<&mut SockOps>::try_into(program) {
                Ok(prog) => match prog.load() {
                    Ok(()) => {
                        // Try safest-to-most-intrusive attach modes so an
                        // existing system program is never replaced if a
                        // non-intrusive mode works. Print Debug form so the
                        // real errno is visible (Aya's Display hides it).
                        // Observed: AllowMultiple -> EINVAL at the root cgroup
                        // on 7.3.4-cachyos; AllowOverride/Single succeed.
                        let mut attached = false;
                        for mode in [
                            ("AllowMultiple", CgroupAttachMode::AllowMultiple),
                            ("AllowOverride", CgroupAttachMode::AllowOverride),
                            ("Single", CgroupAttachMode::Single),
                        ] {
                            match prog.attach(&cgroup, mode.1) {
                                Ok(link) => {
                                    println!("  sockops: attached ({})", mode.0);
                                    _sock_ops_link = Some(link);
                                    attached = true;
                                    break;
                                }
                                Err(error) => {
                                    println!("  sockops {} FAILED: {error:?} — record this", mode.0)
                                }
                            }
                        }
                        if !attached {
                            println!("  sockops: ALL attach modes failed — identity will be 0");
                        }
                    }
                    Err(error) => println!("  sockops load REJECTED: {error:?} — record this"),
                },
                Err(error) => println!("  identity program wrong type: {error}"),
            }
        } else {
            println!("  identity program missing from object");
        }
    } else {
        println!("--no-sock-ops: sock_ops identity disabled");
    }

    println!(
        "\nCollecting for {}s at {}s intervals.",
        args.duration, args.interval
    );

    // Built-in workload: spawn curl so redirects/TLS are handled for us; no
    // second terminal or external setup. curl's size_download is the known
    // truth for the accuracy check.
    let workload_bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let workload_handle = if args.workload {
        println!(
            "Built-in workload: https://{}{} (no second terminal needed)",
            args.workload_host, args.workload_path
        );
        let bytes = std::sync::Arc::clone(&workload_bytes);
        let host = args.workload_host.clone();
        let path = args.workload_path.clone();
        let duration = args.duration;
        Some(std::thread::spawn(move || {
            curl_download(&host, &path, duration, &bytes);
        }))
    } else {
        println!("Start your workload now (Motrix/curl/iperf3) in another terminal.");
        println!(
            "PID {} — CPU/RSS measured by this process; no pidstat needed.",
            std::process::id()
        );
        None
    };

    collect(&mut events, &args, &workload_bytes).context("collection loop")?;
    if let Some(handle) = workload_handle {
        let _ = handle.join();
    }
    Ok(())
}

/// Spawn curl to download the target into /dev/null and read its exact
/// byte count from `-w %{size_download}`. curl handles the HTTPS redirect
/// that a raw std TCP GET cannot. The URL host may include a `:port`, which
/// we drop for the default https URL (curl defaults to 443).
fn curl_download(host: &str, path: &str, duration: u64, total: &std::sync::atomic::AtomicU64) {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::sync::atomic::Ordering;

    let host_only = host.split(':').next().unwrap_or(host);
    let url = format!("https://{host_only}{path}");
    let output = Command::new("curl")
        .args([
            "-sSL",
            "-o",
            "/dev/null",
            "-w",
            "%{size_download}",
            "--max-time",
            &duration.to_string(),
            &url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match output {
        Ok(child) => child,
        Err(error) => {
            eprintln!("workload: failed to spawn curl: {error}");
            return;
        }
    };
    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut out);
    }
    let counted = out.trim().parse::<u64>().unwrap_or(0);
    total.store(counted, Ordering::Relaxed);
    println!(
        "workload: downloaded {counted} bytes ({:.1} MiB)",
        counted as f64 / 1_048_576.0
    );
}

/// Snapshot this process's CPU ticks and RSS pages from /proc (no sysstat).
struct ProcSample {
    ticks: u64,
    rss_pages: u64,
}

fn sample_proc() -> Option<ProcSample> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // comm can contain spaces/parens; fields after the final ')' start at
    // state (field 3). utime/stime are fields 14/15 → indexes 11/12.
    let rest = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let rss_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(ProcSample {
        ticks: utime + stime,
        rss_pages,
    })
}

fn collect(
    events: &mut RingBuf<aya::maps::MapData>,
    args: &Args,
    workload_bytes: &std::sync::atomic::AtomicU64,
) -> Result<()> {
    let started = Instant::now();
    let mut next_print = started + Duration::from_secs(args.interval);
    let mut apps: BTreeMap<u64, AppAgg> = BTreeMap::new();
    let mut total_events: u64 = 0;
    let mut ignored_events: u64 = 0;
    // Interface baseline captured once, so the final summary compares like
    // with like: cumulative attributed bytes vs cumulative interface delta.
    let (base_rx, base_tx) = interface_totals()?;
    let (mut prev_rx, mut prev_tx) = (base_rx, base_tx);
    // Per-interval app deltas, reset at each print, so the live accuracy line
    // compares an interval's attributed bytes with the same interval's
    // interface bytes (the previous code compared cumulative vs interval).
    let (mut interval_rx, mut interval_tx) = (0u64, 0u64);
    let mut proc_prev = sample_proc();
    let mut cpu_pct = 0.0f64;
    let mut max_rss_kib = 0u64;
    // Linux x86_64: USER_HZ is 100 and page size 4096. Only a rough overhead
    // estimate for the spike (Q3), not product code.
    const USER_HZ: f64 = 100.0;
    const PAGE_KIB: u64 = 4;
    let pct = |part: u64, whole: u64| -> f64 {
        if whole == 0 {
            0.0
        } else {
            100.0 * part as f64 / whole as f64
        }
    };

    loop {
        let mut drained = 0usize;
        while let Some(item) = events.next() {
            drained += 1;
            let Some(delta) = Delta::from_bytes(&item) else {
                ignored_events += 1;
                continue;
            };
            total_events += 1;
            let app = apps.entry(delta.cookie).or_default();
            app.pid = delta.pid;
            app.cgroup_id = delta.cgroup_id;
            if delta.source == SRC_KPROBE || delta.source == SRC_SOCK_OPS {
                app.identity_hits += 1;
            } else if delta.source == SRC_CGROUP_SKB {
                if delta.direction == 0 {
                    app.rx += delta.delta_bytes;
                    interval_rx += delta.delta_bytes;
                } else {
                    app.tx += delta.delta_bytes;
                    interval_tx += delta.delta_bytes;
                }
            } else {
                ignored_events += 1;
            }
            app.events += 1;
        }

        if Instant::now() >= next_print {
            let (rx, tx) = interface_totals()?;
            let iface_rx = rx.saturating_sub(prev_rx);
            let iface_tx = tx.saturating_sub(prev_tx);
            prev_rx = rx;
            prev_tx = tx;
            let identified = apps.values().filter(|a| a.identity_hits > 0).count();
            // Self CPU/RSS: no pidstat or sysstat needed (Q3).
            let now_proc = sample_proc();
            if let (Some(prev), Some(now)) = (proc_prev.as_ref(), now_proc.as_ref()) {
                let delta_ticks = now.ticks.saturating_sub(prev.ticks);
                cpu_pct = (delta_ticks as f64 / USER_HZ) / started.elapsed().as_secs_f64().max(1.0)
                    * 100.0;
                max_rss_kib = max_rss_kib.max(now.rss_pages * PAGE_KIB);
            }
            proc_prev = now_proc;
            println!(
                "[+{:>3}s] rx: app {} / iface {} ({:5.1}%)  tx: app {} / iface {} ({:5.1}%)  \
                 cookies {} identified {}/{}  ring {} (drained {} bad {})  cpu {:4.1}% rss {} KiB",
                started.elapsed().as_secs(),
                human(interval_rx),
                human(iface_rx),
                pct(interval_rx, iface_rx),
                human(interval_tx),
                human(iface_tx),
                pct(interval_tx, iface_tx),
                apps.len(),
                identified,
                apps.len(),
                total_events,
                drained,
                ignored_events,
                cpu_pct,
                max_rss_kib,
            );
            interval_rx = 0;
            interval_tx = 0;
            next_print += Duration::from_secs(args.interval);
        }

        if started.elapsed() >= Duration::from_secs(args.duration) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    println!("\n--- final measurement (paste into docs/spike-ebpf-report.md) ---");
    println!("| cookie | pid | cgroup_id | identity_hits | rx_bytes | tx_bytes | events |");
    println!("|---|---:|---:|---:|---:|---:|---:|");
    for (cookie, app) in &apps {
        println!(
            "| {cookie} | {} | {} | {} | {} | {} | {} |",
            app.pid, app.cgroup_id, app.identity_hits, app.rx, app.tx, app.events
        );
    }
    let (iface_now_rx, iface_now_tx) = interface_totals()?;
    let iface_delta_rx = iface_now_rx.saturating_sub(base_rx);
    let iface_delta_tx = iface_now_tx.saturating_sub(base_tx);
    let app_rx: u64 = apps.values().map(|a| a.rx).sum();
    let app_tx: u64 = apps.values().map(|a| a.tx).sum();
    println!(
        "\nattributed rx/tx (cumulative) = {app_rx}/{app_tx}; interface delta over the same \
         window = {iface_delta_rx}/{iface_delta_tx} ({:5.1}% / {:5.1}% attributed); \
         total ring events = {total_events}; bad events = {ignored_events}.",
        pct(app_rx, iface_delta_rx),
        pct(app_tx, iface_delta_tx)
    );
    // Built-in workload's own byte count is the known truth for Q1.
    let workload_total = workload_bytes.load(std::sync::atomic::Ordering::Relaxed);
    if workload_total > 0 {
        println!(
            "built-in workload true bytes = {workload_total}; attributed rx = {app_rx} \
             ({:5.1}% of true) — Q1 accuracy input",
            pct(app_rx, workload_total)
        );
    }
    // An idle run cannot answer the accuracy questions; say so loudly.
    if iface_delta_rx == 0 && iface_delta_tx == 0 && workload_total == 0 {
        println!(
            "WARNING: no interface traffic was seen — was a workload (curl/Motrix/iperf3) \
             running? This run cannot answer Q1/Q2. Re-run with --workload or start traffic."
        );
    }
    println!(
        "Q3 self-measured: peak CPU {:4.1}%, max RSS {} KiB (budget: <=3% CPU, <=150 MB RSS).",
        cpu_pct, max_rss_kib
    );
    println!(
        "Go/no-go inputs: attributed-vs-true <= 5% (curl/iperf3), attributed+unattributed vs \
         interface <= 2%, CPU <= 3%, RSS <= 150 MB."
    );
    Ok(())
}

/// Sum rx/tx bytes of all non-loopback interfaces from local sysfs.
fn interface_totals() -> Result<(u64, u64)> {
    let mut rx = 0u64;
    let mut tx = 0u64;
    for entry in std::fs::read_dir(INTERFACE_ROOT).context("reading sysfs net root")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "lo" || !entry.path().is_dir() {
            continue;
        }
        let dir = entry.path().join("statistics");
        rx += read_u64(&dir.join("rx_bytes"))?;
        tx += read_u64(&dir.join("tx_bytes"))?;
    }
    Ok((rx, tx))
}

fn read_u64(path: &Path) -> Result<u64> {
    let raw = std::fs::read_to_string(path)?;
    raw.trim()
        .parse()
        .with_context(|| format!("parsing {}", path.display()))
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1}{}", UNITS[unit])
}

/// Environment report targeted at the earlier -EPERM blocker.
fn print_environment(cgroup: &Path) {
    println!("--- environment ---");
    println!("arch: {}", std::env::consts::ARCH);
    match std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        Ok(version) => println!("kernel: {}", version.trim()),
        Err(error) => println!("kernel: unknown ({error})"),
    }
    println!(
        "BTF vmlinux: {}",
        if Path::new("/sys/kernel/btf/vmlinux").exists() {
            "present"
        } else {
            "MISSING"
        }
    );

    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let cgroup2: Vec<&str> = mounts
        .lines()
        .filter(|line| line.split_whitespace().nth(1) == Some("cgroup2"))
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    println!(
        "cgroup2 mounts: {}",
        if cgroup2.is_empty() {
            "NONE".to_string()
        } else {
            cgroup2.join(", ")
        }
    );
    println!(
        "requested cgroup: {:?} exists={} cgroup2_marker={} (cgroup.controllers present)",
        cgroup,
        cgroup.exists(),
        cgroup.join("cgroup.controllers").exists()
    );

    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let (eff, bnd) = capabilities(&status);
    for (name, bit) in [
        ("CAP_NET_ADMIN", 12),
        ("CAP_SYS_ADMIN", 21),
        ("CAP_PERFMON", 38),
        ("CAP_BPF", 39),
    ] {
        println!(
            "{name:<15} effective={:<5} bounding={}",
            has_cap(eff, bit),
            has_cap(bnd, bit)
        );
    }
    println!(
        "hint: the earlier C/libbpf spike saw -EPERM because /sys/fs/cgroup was bpffs and \
         CAP_BPF was only in the bounding set. If attach fails again, check those two lines."
    );
}

fn capabilities(status: &str) -> (u64, u64) {
    let mut effective = 0;
    let mut bounding = 0;
    for line in status.lines() {
        if let Some(hex) = line.strip_prefix("CapEff:") {
            effective = u64::from_str_radix(hex.trim(), 16).unwrap_or(0);
        } else if let Some(hex) = line.strip_prefix("CapBnd:") {
            bounding = u64::from_str_radix(hex.trim(), 16).unwrap_or(0);
        }
    }
    (effective, bounding)
}

fn has_cap(set: u64, bit: u32) -> bool {
    set & (1u64 << bit) != 0
}
