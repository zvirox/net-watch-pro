//! Phase B spike runner — loads the Aya programs, attaches on real root, and
//! prints measurements the owner records in docs/spike-ebpf-report.md.
//!
//! Throwaway code: outside the product workspace, never imported by netwatchd.
//! See README.md in this directory for the exact runbook.

use std::collections::BTreeMap;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use aya::include_bytes_aligned;
use aya::maps::RingBuf;
use aya::programs::{CgroupAttachMode, CgroupSkb, CgroupSkbAttachType, SockOps};
use aya::Ebpf;
use clap::Parser;

const INTERFACE_ROOT: &str = "/sys/class/net";
const SRC_CGROUP_SKB: u32 = 1;
const SRC_SOCK_OPS: u32 = 3;
const SRC_LOOPBACK: u32 = 4;
const LOOPBACK_ROUTE_DEST: &str = "00000000";

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

    /// Skip the sock_ops identity probe. Enabled by default.
    #[arg(long)]
    no_sock_ops: bool,

    /// Interface whose sysfs counters are compared with attributed bytes.
    /// Default: interface of the default route.
    #[arg(long)]
    iface: Option<String>,

    /// Generate our own known-rate download during collection.
    #[arg(long)]
    workload: bool,

    /// Host:port for --workload's HTTP server
    #[arg(long, default_value = "proof.ovh.net:80")]
    workload_host: String,

    /// Path for --workload's HTTP GET
    #[arg(long, default_value = "/files/100Mb.dat")]
    workload_path: String,

    /// Print environment diagnostics only; do not load or attach eBPF
    #[arg(long)]
    env_only: bool,
}

/// Identical layout to `Delta` in the eBPF program (`#[repr(C)]`,
/// little-endian): cookie u64@0, pid u32@8, tid u32@12, cgroup u64@16,
/// bytes u64@24, direction u32@32, source u32@36, comm [u8;16]@40 (56 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Delta {
    cookie: u64,
    pid: u32,
    tid: u32,
    cgroup_id: u64,
    delta_bytes: u64,
    direction: u32,
    source: u32,
    comm: [u8; 16],
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
        let mut comm = [0u8; 16];
        comm.copy_from_slice(&bytes[40..56]);
        Some(Self {
            cookie: u64_at(0),
            pid: u32_at(8),
            tid: u32_at(12),
            cgroup_id: u64_at(16),
            delta_bytes: u64_at(24),
            direction: u32_at(32),
            source: u32_at(36),
            comm,
        })
    }
}

#[derive(Default, Debug)]
struct AppAgg {
    pid: u32,
    cgroup_id: u64,
    comm: String,
    cgroup_path: String,
    rx: u64,
    tx: u64,
    loopback_rx: u64,
    loopback_tx: u64,
    identity_hits: u32,
    events: u64,
}

fn comm_string(bytes: &[u8; 16]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(16);
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn main() -> Result<()> {
    let args = Args::parse();
    print_environment(&args.cgroup);

    // Resolve the interface to compare against before touching eBPF.
    let iface = match &args.iface {
        Some(name) => name.clone(),
        None => default_route_iface()?
            .ok_or_else(|| anyhow!("no default route found — pass --iface <name> explicitly"))?,
    };
    println!("comparison interface: {iface}");

    if args.env_only {
        return Ok(());
    }

    println!("\n--- load ---");
    let bytes = include_bytes_aligned!(concat!(env!("OUT_DIR"), "/netwatch-spike-ebpf"));
    let mut bpf = Ebpf::load(bytes).context("loading eBPF object (is this real root with BTF?)")?;

    // Write the loopback ifindex into CONFIG so the BPF side can filter by
    // number, not by name.
    let lo_ifindex = if_nametoindex("lo")?;
    let config_map = bpf
        .take_map("CONFIG")
        .ok_or_else(|| anyhow!("CONFIG map missing from object"))?;
    let mut config: aya::maps::Array<_, u32> = config_map.try_into().context("opening CONFIG")?;
    config
        .set(0, &lo_ifindex, 0)
        .context("writing lo ifindex")?;
    println!("loopback ifindex: {lo_ifindex} (filtered out of attributed bytes)");

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

    // Hold every link to the end of main so programs stay attached, then
    // drop them explicitly for a clean detach.
    // Declared uninitialized: attach failure calls bail!(), so every path
    // reaching the final drop has assigned them.
    let ingress_link;
    let egress_link;
    let mut sock_ops_link = None;

    println!("attaching cgroup_skb ingress/egress to {:?}", args.cgroup);
    {
        let ingress: &mut CgroupSkb = bpf
            .program_mut("ingress")
            .ok_or_else(|| anyhow!("ingress program missing"))?
            .try_into()?;
        ingress.load().context("loading ingress cgroup_skb")?;
        match ingress.attach(
            &cgroup,
            CgroupSkbAttachType::Ingress,
            CgroupAttachMode::Single,
        ) {
            Ok(link) => {
                ingress_link = Some(link);
                println!("  ingress: attached");
            }
            Err(error) => bail!("ingress attach failed: {error}"),
        }
    }
    {
        let egress: &mut CgroupSkb = bpf
            .program_mut("egress")
            .ok_or_else(|| anyhow!("egress program missing"))?
            .try_into()?;
        egress.load().context("loading egress cgroup_skb")?;
        match egress.attach(
            &cgroup,
            CgroupSkbAttachType::Egress,
            CgroupAttachMode::Single,
        ) {
            Ok(link) => {
                egress_link = Some(link);
                println!("  egress: attached");
            }
            Err(error) => bail!("egress attach failed: {error}"),
        }
    }

    if !args.no_sock_ops {
        println!("attaching sock_ops identity probe:");
        if let Some(program) = bpf.program_mut("identity") {
            match TryInto::<&mut SockOps>::try_into(program) {
                Ok(prog) => match prog.load() {
                    Ok(()) => {
                        // Pre-flight: always print the cgroup tree and check
                        // for an existing sock_ops program BEFORE attaching.
                        let existing = cgroup_tree_sock_ops(&args.cgroup)?;
                        if existing {
                            println!("  existing sock_ops program found — refusing Single attach");
                        }
                        // AllowMultiple is the only non-intrusive mode. If it
                        // fails and another program exists, never override it.
                        match prog.attach(&cgroup, CgroupAttachMode::AllowMultiple) {
                            Ok(link) => {
                                println!("  sockops: attached (AllowMultiple)");
                                sock_ops_link = Some(link);
                            }
                            Err(multi_error) => {
                                println!("  sockops AllowMultiple FAILED: {multi_error:?}");
                                if existing {
                                    bail!(
                                        "REFUSING to attach sock_ops with Single: another \
                                         sock_ops program is already attached at {:?}. \
                                         Detach it first or run with --no-sock-ops; the spike \
                                         must not override it.",
                                        args.cgroup
                                    );
                                }
                                match prog.attach(&cgroup, CgroupAttachMode::Single) {
                                    Ok(link) => {
                                        println!(
                                            "  sockops: attached (Single; no existing program found)"
                                        );
                                        sock_ops_link = Some(link);
                                    }
                                    Err(single_error) => println!(
                                        "  sockops Single FAILED: {single_error:?} — identity will be 0"
                                    ),
                                }
                            }
                        }
                    }
                    Err(error) => println!("  sockops load REJECTED: {error:?}"),
                },
                Err(error) => println!("  identity program wrong type: {error}"),
            }
        } else {
            println!("  identity program missing from object");
        }
    } else {
        println!("--no-sock-ops: identity disabled");
    }

    println!(
        "\nCollecting for {}s at {}s intervals.",
        args.duration, args.interval
    );

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

    let result = collect(&mut events, &args, &iface, lo_ifindex, &workload_bytes);

    if let Some(handle) = workload_handle {
        let _ = handle.join();
    }
    // Explicit clean detach: dropping the links detaches the programs.
    drop(sock_ops_link);
    drop(egress_link);
    drop(ingress_link);
    println!("\nall eBPF programs detached.");

    result
}

/// Print the `bpftool cgroup tree` output for this cgroup and report whether
/// a sock_ops program is already attached. This is the equivalent of the
/// owner's `sudo bpftool cgroup tree` pre-flight, run automatically before we
/// attach anything. If bpftool fails we refuse to continue (conservative).
fn cgroup_tree_sock_ops(cgroup: &Path) -> Result<bool> {
    let path = cgroup.to_string_lossy();
    println!("  bpftool cgroup tree {path}:");
    let output = Command::new("bpftool")
        .args(["cgroup", "tree", path.as_ref()])
        .output()
        .context("running `bpftool cgroup tree` (is bpftool installed?)")?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        println!("    {line}");
    }
    if !output.status.success() {
        bail!(
            "`bpftool cgroup tree` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(text.lines().any(|line| line.contains("sock_ops")))
}

/// Interface of the default route from /proc/net/route (destination 00000000).
fn default_route_iface() -> Result<Option<String>> {
    let raw = std::fs::read_to_string("/proc/net/route")?;
    for line in raw.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() > 1 && fields[1] == LOOPBACK_ROUTE_DEST {
            return Ok(Some(fields[0].to_string()));
        }
    }
    Ok(None)
}

/// Resolve the ifindex of an interface by sysfs (avoids a libc dependency).
fn if_nametoindex(name: &str) -> Result<u32> {
    let path = Path::new(INTERFACE_ROOT).join(name).join("ifindex");
    read_u64(&path)
        .with_context(|| format!("reading ifindex for {name}"))?
        .try_into()
        .context("ifindex does not fit u32")
}

/// Sum rx/tx bytes of ONE interface from sysfs (the comparison interface).
fn interface_totals(iface: &str) -> Result<(u64, u64)> {
    let dir = Path::new(INTERFACE_ROOT).join(iface).join("statistics");
    Ok((
        read_u64(&dir.join("rx_bytes"))?,
        read_u64(&dir.join("tx_bytes"))?,
    ))
}

/// Loopback counters, reported separately (never compared with the NIC).
fn loopback_totals() -> Result<(u64, u64)> {
    interface_totals("lo")
}

fn curl_download(host: &str, path: &str, duration: u64, total: &std::sync::atomic::AtomicU64) {
    use std::io::Read;
    use std::process::Stdio;
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

struct ProcSample {
    ticks: u64,
    rss_pages: u64,
}

fn sample_proc() -> Option<ProcSample> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
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

/// Read the cgroup v2 path for a PID from /proc/<pid>/cgroup. This is the
/// authoritative source for "which app scope is this socket in", better than
/// the inode walk because it names the systemd scope directly.
fn cgroup_path_for_pid(pid: u32) -> Option<String> {
    let raw = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    for line in raw.lines() {
        let mut fields = line.split(':');
        let hierarchy = fields.next()?;
        let path = fields.nth(1)?; // fields: hierarchy : controllers : path
        if hierarchy == "0" && !path.is_empty() && path != "/" {
            return Some(path.to_string());
        }
    }
    None
}

/// Resolve a cgroup v2 id (inode number) to its path under the cgroup mount.
fn resolve_cgroup_path(cgroup_id: u64) -> Option<String> {
    if cgroup_id == 0 || cgroup_id == 1 {
        return None;
    }
    fn walk(dir: &Path, target: u64, depth: u32) -> Option<String> {
        if depth > 5 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if entry.metadata().ok()?.ino() == target {
                return Some(path.to_string_lossy().into_owned());
            }
            if let Some(found) = walk(&path, target, depth + 1) {
                return Some(found);
            }
        }
        None
    }
    walk(Path::new("/sys/fs/cgroup"), cgroup_id, 0)
}

#[allow(clippy::too_many_arguments)]
fn collect(
    events: &mut RingBuf<aya::maps::MapData>,
    args: &Args,
    iface: &str,
    _lo_ifindex: u32,
    workload_bytes: &std::sync::atomic::AtomicU64,
) -> Result<()> {
    let started = Instant::now();
    let mut next_print = started + Duration::from_secs(args.interval);
    let mut apps: BTreeMap<u64, AppAgg> = BTreeMap::new();
    let mut total_events: u64 = 0;
    let mut ignored_events: u64 = 0;
    let (base_rx, base_tx) = interface_totals(iface)?;
    let (base_lo_rx, base_lo_tx) = loopback_totals()?;
    let (mut prev_rx, mut prev_tx) = (base_rx, base_tx);
    let (mut interval_rx, mut interval_tx) = (0u64, 0u64);
    let mut proc_prev = sample_proc();
    let mut cpu_pct = 0.0f64;
    let mut max_rss_kib = 0u64;
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
        while let Some(item) = events.next() {
            let Some(delta) = Delta::from_bytes(&item) else {
                ignored_events += 1;
                continue;
            };
            total_events += 1;
            let app = apps.entry(delta.cookie).or_default();
            if delta.source == SRC_SOCK_OPS {
                // Authoritative identity from connect()/accept().
                app.pid = delta.pid;
                app.cgroup_id = delta.cgroup_id;
                app.comm = comm_string(&delta.comm);
                app.cgroup_path = cgroup_path_for_pid(delta.pid)
                    .or_else(|| resolve_cgroup_path(delta.cgroup_id))
                    .unwrap_or_default();
                app.identity_hits += 1;
            } else if delta.source == SRC_CGROUP_SKB {
                if delta.pid != 0 && app.pid == 0 {
                    app.pid = delta.pid;
                    app.cgroup_id = delta.cgroup_id;
                    app.comm = comm_string(&delta.comm);
                    app.cgroup_path = cgroup_path_for_pid(delta.pid)
                        .or_else(|| resolve_cgroup_path(delta.cgroup_id))
                        .unwrap_or_default();
                }
                if delta.direction == 0 {
                    app.rx += delta.delta_bytes;
                    interval_rx += delta.delta_bytes;
                } else {
                    app.tx += delta.delta_bytes;
                    interval_tx += delta.delta_bytes;
                }
            } else if delta.source == SRC_LOOPBACK {
                if delta.direction == 0 {
                    app.loopback_rx += delta.delta_bytes;
                } else {
                    app.loopback_tx += delta.delta_bytes;
                }
            } else {
                ignored_events += 1;
            }
            app.events += 1;
        }

        if Instant::now() >= next_print {
            let (rx, tx) = interface_totals(iface)?;
            let iface_rx = rx.saturating_sub(prev_rx);
            let iface_tx = tx.saturating_sub(prev_tx);
            prev_rx = rx;
            prev_tx = tx;
            let identified = apps
                .values()
                .filter(|a| a.pid != 0 && a.cgroup_id != 1)
                .count();
            let now_proc = sample_proc();
            if let (Some(prev), Some(now)) = (proc_prev.as_ref(), now_proc.as_ref()) {
                let delta_ticks = now.ticks.saturating_sub(prev.ticks);
                cpu_pct = (delta_ticks as f64 / USER_HZ) / started.elapsed().as_secs_f64().max(1.0)
                    * 100.0;
                max_rss_kib = max_rss_kib.max(now.rss_pages * PAGE_KIB);
            }
            proc_prev = now_proc;
            println!(
                "[+{:>3}s] rx: app {} / iface({}) {} ({:5.1}%)  tx: app {} / iface {} ({:5.1}%)  \
                 identified {}/{}  ring {} (bad {})  cpu {:4.1}% rss {} KiB",
                started.elapsed().as_secs(),
                human(interval_rx),
                iface,
                human(iface_rx),
                pct(interval_rx, iface_rx),
                human(interval_tx),
                human(iface_tx),
                pct(interval_tx, iface_tx),
                identified,
                apps.len(),
                total_events,
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

    // ---- final measurement ----
    println!("\n--- final measurement (paste into docs/spike-ebpf-report.md) ---");

    // Aggregate by app identity (pid + cgroup), not by cookie.
    let mut by_app: BTreeMap<String, AppAgg> = BTreeMap::new();
    for app in apps.values() {
        // cgroup_id 1 is the root cgroup: not an app identity, so treat it
        // as UNIDENTIFIED even though a pid was captured.
        let is_app = app.pid != 0 && app.cgroup_id != 1;
        let key = if is_app {
            let name = if !app.cgroup_path.is_empty() {
                Path::new(&app.cgroup_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| app.cgroup_path.clone())
            } else if !app.comm.is_empty() {
                app.comm.clone()
            } else {
                format!("pid-{}", app.pid)
            };
            format!("{} (pid {}, cg {})", name, app.pid, app.cgroup_id)
        } else {
            "UNIDENTIFIED".to_string()
        };
        let agg = by_app.entry(key).or_default();
        agg.pid = agg.pid.max(app.pid);
        agg.cgroup_id = agg.cgroup_id.max(app.cgroup_id);
        if agg.comm.is_empty() {
            agg.comm = app.comm.clone();
        }
        if agg.cgroup_path.is_empty() {
            agg.cgroup_path = app.cgroup_path.clone();
        }
        agg.rx += app.rx;
        agg.tx += app.tx;
        agg.loopback_rx += app.loopback_rx;
        agg.loopback_tx += app.loopback_tx;
        agg.identity_hits += app.identity_hits;
        agg.events += app.events;
    }

    let app_rx: u64 = by_app.values().map(|a| a.rx).sum();
    let app_tx: u64 = by_app.values().map(|a| a.tx).sum();
    let loopback_rx: u64 = by_app.values().map(|a| a.loopback_rx).sum();
    let loopback_tx: u64 = by_app.values().map(|a| a.loopback_tx).sum();

    println!(
        "| app / cgroup | pid | cgroup_id | identity_hits | rx_bytes | tx_bytes | rx share | tx share | events |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (name, app) in &by_app {
        println!(
            "| {name} | {} | {} | {} | {} | {} | {:5.1}% | {:5.1}% | {} |",
            app.pid,
            app.cgroup_id,
            app.identity_hits,
            app.rx,
            app.tx,
            pct(app.rx, app_rx),
            pct(app.tx, app_tx),
            app.events
        );
    }

    let (iface_now_rx, iface_now_tx) = interface_totals(iface)?;
    let iface_delta_rx = iface_now_rx.saturating_sub(base_rx);
    let iface_delta_tx = iface_now_tx.saturating_sub(base_tx);
    let (lo_now_rx, lo_now_tx) = loopback_totals()?;
    let lo_delta_rx = lo_now_rx.saturating_sub(base_lo_rx);
    let lo_delta_tx = lo_now_tx.saturating_sub(base_lo_tx);

    println!(
        "\nattributed rx/tx (non-loopback) = {app_rx}/{app_tx}; interface({iface}) delta = \
         {iface_delta_rx}/{iface_delta_tx} => {:.1}% / {:.1}% attributed (error {:.1}% / {:.1}%).",
        pct(app_rx, iface_delta_rx),
        pct(app_tx, iface_delta_tx),
        100.0 - pct(app_rx, iface_delta_rx),
        100.0 - pct(app_tx, iface_delta_tx),
    );
    println!(
        "loopback (reported separately, NOT compared): cgroup_skb {loopback_rx}/{loopback_tx} B, \
         sysfs lo delta {lo_delta_rx}/{lo_delta_tx} B."
    );
    println!(
        "identified cookies: {} / {}; ring events {}; bad {}.",
        by_app
            .values()
            .filter(|a| a.pid != 0 && a.cgroup_id != 1)
            .count(),
        by_app.len(),
        total_events,
        ignored_events
    );

    let workload_total = workload_bytes.load(std::sync::atomic::Ordering::Relaxed);
    if workload_total > 0 {
        println!(
            "built-in workload true bytes = {workload_total}; attributed rx = {app_rx} \
             ({:5.1}% of true) — accuracy input when it is the only workload.",
            pct(app_rx, workload_total)
        );
    }

    println!(
        "Q3 self-measured: peak CPU {:4.1}%, max RSS {} KiB (budget: <=3% CPU, <=150 MB RSS).",
        cpu_pct, max_rss_kib
    );
    println!(
        "Go/no-go: |error| <= 5% on a single transfer; attributed+unattributed vs interface <= 2%; \
         CPU <= 3%; RSS <= 150 MB. Mark Q1-Q10 VERIFIED only when measured."
    );
    Ok(())
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

/// Environment report: cgroup2 detection from /proc/self/mountinfo (the
/// previous /proc/mounts check looked at the mountpoint field by mistake).
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

    // mountinfo format: ... mount point ... - fstype source superopts
    let mountinfo = std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
    let cgroup2: Vec<String> = mountinfo
        .lines()
        .filter_map(|line| {
            let sep = line.find(" - ")?;
            let tail: Vec<&str> = line[sep + 3..].split_whitespace().collect();
            if tail.first() != Some(&"cgroup2") {
                return None;
            }
            let head: Vec<&str> = line[..sep].split_whitespace().collect();
            head.get(4).map(|s| (*s).to_string())
        })
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
