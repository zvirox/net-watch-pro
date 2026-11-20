//! Phase B spike runner — loads the Aya programs, attaches on real root, and
//! prints measurements the owner records in docs/spike-ebpf-report.md.
//!
//! Throwaway code: outside the product workspace, never imported by netwatchd.
//! See README.md in this directory for the exact runbook.

use std::collections::{BTreeMap, HashMap};
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
const ROOT_CGROUP: &str = "/sys/fs/cgroup";
const SRC_CGROUP_SKB: u32 = 1;
const SRC_SOCK_OPS: u32 = 3;
const SRC_LOOPBACK: u32 = 4;
const LOOPBACK_ROUTE_DEST: &str = "00000000";

#[derive(Parser, Debug)]
#[command(name = "netwatch-spike", about = "Phase B eBPF accounting spike (Aya)")]
struct Args {
    /// cgroup v2 directory to attach cgroup_skb programs to
    #[arg(long, default_value = ROOT_CGROUP)]
    cgroup: PathBuf,

    /// Attach at the cgroup v2 ROOT (/sys/fs/cgroup) regardless of --cgroup,
    /// and print an Unattributed row (interface delta minus attributed bytes).
    #[arg(long)]
    root_attach: bool,

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
/// little-endian): cookie u64@0, cgroup_id u64@8, pid u32@16, tid u32@20,
/// bytes u64@24, direction u32@32, source u32@36, comm [u8;16]@40 (56 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Delta {
    cookie: u64,
    cgroup_id: u64,
    pid: u32,
    tid: u32,
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
            cgroup_id: u64_at(8),
            pid: u32_at(16),
            tid: u32_at(20),
            delta_bytes: u64_at(24),
            direction: u32_at(32),
            source: u32_at(36),
            comm,
        })
    }
}

#[derive(Default, Debug)]
struct AppAgg {
    /// Debug only: 'current' in softirq, not an identity.
    debug_pid: u32,
    debug_comm: String,
    cgroup_path: String,
    display: String,
    confidence: String,
    rx: u64,
    tx: u64,
    loopback_rx: u64,
    loopback_tx: u64,
    /// One Delta = one packet; used to test L2-header/per-packet tx-gap theory.
    rx_packets: u64,
    tx_packets: u64,
    identity_hits: u32,
    events: u64,
}

/// cgroupfs inode -> path index, with throttled rescan for unknown ids.
struct CgroupIndex {
    paths: HashMap<u64, String>,
    last_scan: Instant,
}

impl CgroupIndex {
    fn new() -> Self {
        let mut me = Self {
            paths: HashMap::new(),
            last_scan: Instant::now() - Duration::from_secs(60),
        };
        me.scan();
        me
    }

    fn scan(&mut self) {
        fn walk(dir: &Path, out: &mut HashMap<u64, String>, depth: u32) {
            if depth > 8 {
                return;
            }
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    out.insert(meta.ino(), path.to_string_lossy().into_owned());
                }
                walk(&path, out, depth + 1);
            }
        }
        let mut found = HashMap::new();
        walk(Path::new(ROOT_CGROUP), &mut found, 0);
        self.paths = found;
        self.last_scan = Instant::now();
    }

    fn path(&mut self, cgroup_id: u64) -> Option<String> {
        if cgroup_id <= 1 {
            return None;
        }
        if let Some(path) = self.paths.get(&cgroup_id) {
            return Some(path.clone());
        }
        // Unknown id: rescan at most once per second, then retry once.
        if self.last_scan.elapsed() >= Duration::from_secs(1) {
            self.scan();
            return self.paths.get(&cgroup_id).cloned();
        }
        None
    }
}

/// Derive (display name, confidence) from a cgroup path.
/// - app-<name>-<pid>.scope / app-<name>@<hash>.service -> <name> (high)
/// - match a .desktop file by Exec/stem -> the desktop Name (high)
/// - session-* / vte-spawn-* -> debug exe name with confidence "low"
fn display_for(path: &str, debug_comm: &str, desktop: &[(String, String)]) -> (String, String) {
    if path.is_empty() || path == "/" {
        return ("UNIDENTIFIED".to_string(), "none".to_string());
    }
    let leaf = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());

    // systemd app scope: app-motrix-31433.scope or app-motrix@<hash>.service
    if let Some(rest) = leaf.strip_prefix("app-") {
        let mut name = rest.to_string();
        for suffix in [".scope", ".service"] {
            if let Some(stripped) = name.strip_suffix(suffix) {
                name = stripped.to_string();
            }
        }
        // strip a trailing -<pid> or @<hash>
        if let Some(idx) = name.rfind(|c: char| c == '-' || c == '@') {
            let tail = &name[idx + 1..];
            if tail
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c.is_ascii_digit())
            {
                name = name[..idx].to_string();
            }
        }
        if !name.is_empty() {
            // Prefer a .desktop display name when one matches.
            for (hint, desktop_name) in desktop {
                if name.eq_ignore_ascii_case(hint) {
                    return (desktop_name.clone(), "high".to_string());
                }
            }
            return (name, "high".to_string());
        }
    }

    // .desktop match directly on the leaf name.
    for (hint, desktop_name) in desktop {
        if leaf.eq_ignore_ascii_case(hint) {
            return (desktop_name.clone(), "high".to_string());
        }
    }

    // session-*/vte-spawn-* shells: no app identity; fall back to exe/comm.
    if leaf.starts_with("session-") || leaf.starts_with("vte-spawn-") {
        let fallback = if debug_comm.is_empty() {
            "unknown-shell".to_string()
        } else {
            debug_comm.to_string()
        };
        return (fallback, "low".to_string());
    }

    (leaf, "low".to_string())
}

/// Collect (hint, Name) pairs from .desktop files for app display names.
fn desktop_names() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(Path::new(&home).join(".local/share/applications"));
    }
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let mut name = None;
            let mut exec = None;
            for line in text.lines() {
                if line.starts_with("Name=") && name.is_none() {
                    name = Some(line[5..].trim().to_string());
                } else if line.starts_with("Exec=") && exec.is_none() {
                    exec = Some(line[5..].trim().to_string());
                }
            }
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(exe) = exec {
                let first = exe.split_whitespace().next().unwrap_or("");
                let exe_hint = Path::new(first)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| first.to_string());
                if let Some(display) = &name {
                    out.push((exe_hint, display.clone()));
                }
            }
            if let Some(display) = name {
                out.push((stem, display));
            }
        }
    }
    out
}

fn comm_string(bytes: &[u8; 16]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(16);
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn main() -> Result<()> {
    let args = Args::parse();
    let attach_path = if args.root_attach {
        PathBuf::from(ROOT_CGROUP)
    } else {
        args.cgroup.clone()
    };
    if args.root_attach {
        println!("--root-attach: attaching at {ROOT_CGROUP} (per-app rows + Unattributed)");
    }
    print_environment(&attach_path);

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

    let cgroup = File::open(&attach_path).with_context(|| {
        format!("opening cgroup {attach_path:?} — use --cgroup to point at a cgroup2 mount")
    })?;

    let ingress_link;
    let egress_link;
    let mut sock_ops_link = None;

    println!("attaching cgroup_skb ingress/egress to {attach_path:?}");
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
                        let existing = cgroup_tree_sock_ops(&attach_path)?;
                        if existing {
                            println!("  existing sock_ops program found — refusing Single attach");
                        }
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
                                         sock_ops program is already attached at {attach_path:?}. \
                                         Detach it first or run with --no-sock-ops."
                                    );
                                }
                                match prog.attach(&cgroup, CgroupAttachMode::Single) {
                                    Ok(link) => {
                                        println!("  sockops: attached (Single; no existing program found)");
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
    drop(sock_ops_link);
    drop(egress_link);
    drop(ingress_link);
    println!("\nall eBPF programs detached.");

    result
}

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

fn if_nametoindex(name: &str) -> Result<u32> {
    let path = Path::new(INTERFACE_ROOT).join(name).join("ifindex");
    read_u64(&path)
        .with_context(|| format!("reading ifindex for {name}"))?
        .try_into()
        .context("ifindex does not fit u32")
}

fn interface_totals(iface: &str) -> Result<(u64, u64)> {
    let dir = Path::new(INTERFACE_ROOT).join(iface).join("statistics");
    Ok((
        read_u64(&dir.join("rx_bytes"))?,
        read_u64(&dir.join("tx_bytes"))?,
    ))
}

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
    // AGGREGATION KEY = socket cgroup id (NOT pid, NOT cookie).
    let mut apps: BTreeMap<u64, AppAgg> = BTreeMap::new();
    let mut cgroups = CgroupIndex::new();
    let desktop = desktop_names();
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
            let key = delta.cgroup_id;
            let app = apps.entry(key).or_default();
            if delta.pid != 0 && app.debug_pid == 0 {
                app.debug_pid = delta.pid;
                app.debug_comm = comm_string(&delta.comm);
            }
            if delta.source == SRC_SOCK_OPS {
                app.identity_hits += 1;
            } else if delta.source == SRC_CGROUP_SKB {
                if delta.direction == 0 {
                    app.rx += delta.delta_bytes;
                    app.rx_packets += 1;
                    interval_rx += delta.delta_bytes;
                } else {
                    app.tx += delta.delta_bytes;
                    app.tx_packets += 1;
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

            // Resolve path + display name lazily (with rescan for unknown ids).
            if app.display.is_empty() {
                if let Some(path) = cgroups.path(key) {
                    app.cgroup_path = path;
                }
                let (display, confidence) =
                    display_for(&app.cgroup_path, &app.debug_comm, &desktop);
                app.display = display;
                app.confidence = confidence;
            }
        }

        if Instant::now() >= next_print {
            let (rx, tx) = interface_totals(iface)?;
            let iface_rx = rx.saturating_sub(prev_rx);
            let iface_tx = tx.saturating_sub(prev_tx);
            prev_rx = rx;
            prev_tx = tx;
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
                 cgroups {}  ring {} (bad {})  cpu {:4.1}% rss {} KiB",
                started.elapsed().as_secs(),
                human(interval_rx),
                iface,
                human(iface_rx),
                pct(interval_rx, iface_rx),
                human(interval_tx),
                human(iface_tx),
                pct(interval_tx, iface_tx),
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
    println!("aggregation key: socket cgroup id (bpf_skb_cgroup_id); pid is debug only");
    if args.root_attach {
        println!("mode: root attach — an Unattributed row is included");
    }

    let app_rx: u64 = apps.values().map(|a| a.rx).sum();
    let app_tx: u64 = apps.values().map(|a| a.tx).sum();
    let rx_packets: u64 = apps.values().map(|a| a.rx_packets).sum();
    let tx_packets: u64 = apps.values().map(|a| a.tx_packets).sum();
    let loopback_rx: u64 = apps.values().map(|a| a.loopback_rx).sum();
    let loopback_tx: u64 = apps.values().map(|a| a.loopback_tx).sum();

    println!(
        "| app (display) | confidence | cgroup id | cgroup path | rx_bytes | tx_bytes | rx share | tx share | debug pid/comm | events |"
    );
    println!("|---|---|---:|---|---:|---:|---:|---:|---|---:|");
    let mut sorted: Vec<(&u64, &AppAgg)> = apps.iter().collect();
    sorted.sort_by_key(|(_, a)| std::cmp::Reverse(a.rx + a.tx));
    for (cgroup_id, app) in sorted {
        println!(
            "| {} | {} | {} | {} | {} | {} | {:5.1}% | {:5.1}% | {}/{} | {} |",
            app.display,
            app.confidence,
            cgroup_id,
            if app.cgroup_path.is_empty() {
                "-"
            } else {
                &app.cgroup_path
            },
            app.rx,
            app.tx,
            pct(app.rx, app_rx),
            pct(app.tx, app_tx),
            app.debug_pid,
            if app.debug_comm.is_empty() {
                "-"
            } else {
                &app.debug_comm
            },
            app.events
        );
    }

    let (iface_now_rx, iface_now_tx) = interface_totals(iface)?;
    let iface_delta_rx = iface_now_rx.saturating_sub(base_rx);
    let iface_delta_tx = iface_now_tx.saturating_sub(base_tx);
    let unattr_rx = iface_delta_rx.saturating_sub(app_rx);
    let unattr_tx = iface_delta_tx.saturating_sub(app_tx);
    if args.root_attach {
        println!(
            "| Unattributed | - | - | interface delta minus attributed | {} | {} | {:5.1}% | {:5.1}% | - | - |",
            unattr_rx,
            unattr_tx,
            pct(unattr_rx, iface_delta_rx),
            pct(unattr_tx, iface_delta_tx)
        );
    }

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
    if args.root_attach {
        println!(
            "unattributed rx/tx = {unattr_rx}/{unattr_tx} ({:5.1}% / {:5.1}% of interface delta)",
            pct(unattr_rx, iface_delta_rx),
            pct(unattr_tx, iface_delta_tx)
        );
    }
    // tx-gap diagnostic: one Delta = one packet, so bytes/packet on each side
    // can be compared. A ~14 B/packet difference would support the L2-header
    // hypothesis; this prints the numbers, it does NOT assert the cause.
    let avg_app_tx = if tx_packets == 0 {
        0.0
    } else {
        app_tx as f64 / tx_packets as f64
    };
    let avg_iface_tx = if tx_packets == 0 {
        0.0
    } else {
        iface_delta_tx as f64 / tx_packets as f64
    };
    println!(
        "tx-gap diagnostic: cgroup_skb tx packets {tx_packets}, app {:.1} B/pkt vs \
         iface({iface}) {:.1} B/pkt (delta {:.1} B/pkt over {} packets). \
         Compare ~14 B Ethernet header; cause UNMEASURED until checked.",
        avg_app_tx,
        avg_iface_tx,
        avg_iface_tx - avg_app_tx,
        tx_packets
    );
    println!(
        "rx diagnostic: cgroup_skb rx packets {rx_packets}, app {:.1} B/pkt.",
        if rx_packets == 0 {
            0.0
        } else {
            app_rx as f64 / rx_packets as f64
        }
    );
    println!(
        "loopback (reported separately, NOT compared): cgroup_skb {loopback_rx}/{loopback_tx} B, \
         sysfs lo delta {lo_delta_rx}/{lo_delta_tx} B."
    );
    println!(
        "cgroups {}; ring events {}; bad {}; sock_ops identity hits {}.",
        apps.len(),
        total_events,
        ignored_events,
        apps.values().map(|a| a.identity_hits).sum::<u32>()
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
