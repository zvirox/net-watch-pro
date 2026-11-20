//! Phase C (C1–C3): eBPF collector for per-app byte attribution.
//!
//! Loads the separately built object, attaches `cgroup_skb` at the cgroup v2
//! root with the bounded capability set, and samples the per-cgroup byte map
//! once per second. Never panics: a missing object or failed attach is a
//! degraded state (ARCHITECTURE.md §7.1), not a startup failure.
//!
//! Identity comes from the socket's cgroup (`bpf_skb_cgroup_id` in the BPF
//! program), never from `current` — see docs/spike-ebpf-report.md §Twelfth.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

use aya::maps::{Array, HashMap as AyaHashMap, MapData};
use aya::programs::{CgroupAttachMode, CgroupSkb, CgroupSkbAttachType};
use aya::{Ebpf, Pod};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use netwatch_common::{AppUsage, AppUsageFrame, CollectorStats, RateSnapshot};

const DEFAULT_OBJECT: &str = "/usr/lib/netwatch-pro/netwatch_ebpf.o";
const ROOT_CGROUP: &str = "/sys/fs/cgroup";
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// Mirror of `Counts` in the BPF program. Must stay repr(C)-compatible.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Counts {
    rx_bytes: u64,
    tx_bytes: u64,
    packets: u32,
}
// SAFETY: repr(C) struct of integer fields, no padding at these alignments.
#[allow(unsafe_code)]
unsafe impl Pod for Counts {}

/// Receiver of the latest per-app rows (fed to the `live.apps` stream).
pub type AppUsageRx = watch::Receiver<std::sync::Arc<AppUsageFrame>>;

/// Outcome of the loader, used to set the honest status snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorState {
    /// Object loaded and programs attached.
    Attached,
    /// Explicitly disabled or object missing — collector not running.
    Disabled,
    /// Load or attach failed.
    Failed,
}

/// Start the collector if enabled. Returns state + a watch receiver of the
/// latest per-app byte snapshot. Never returns an error for a missing object.
pub fn start(
    disabled: bool,
    object_override: Option<PathBuf>,
    rates: watch::Receiver<std::sync::Arc<RateSnapshot>>,
) -> (CollectorState, AppUsageRx, Option<JoinHandle<()>>) {
    let empty = std::sync::Arc::new(AppUsageFrame::default());
    let (tx, rx) = watch::channel(empty);
    if disabled {
        tracing::info!("NETWATCH_DISABLE_EBPF set: collector disabled");
        return (CollectorState::Disabled, rx, None);
    }
    let object = object_override.unwrap_or_else(|| PathBuf::from(DEFAULT_OBJECT));
    let bytes = match std::fs::read(&object) {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(path = %object.display(), %error, "eBPF object unavailable; collector degraded");
            return (CollectorState::Disabled, rx, None);
        }
    };

    let mut bpf = match Ebpf::load(&bytes) {
        Ok(bpf) => bpf,
        Err(error) => {
            tracing::warn!(%error, "eBPF object load failed; collector degraded");
            return (CollectorState::Failed, rx, None);
        }
    };

    // Write lo's ifindex so the BPF side can filter loopback by number.
    // Use map_mut (borrow) rather than take_map (move): take_map closes the
    // fd on drop, leaving the program's map relocation pointing at an
    // invalid fd ("fd N is not pointing to valid bpf_map" on BPF_PROG_LOAD).
    if let Some(lo_index) = lo_ifindex()
        && let Some(map) = bpf.map_mut("CONFIG")
        && let Ok(mut config) = Array::<&mut MapData, u32>::try_from(map)
        && let Err(error) = config.set(0, &lo_index, 0)
    {
        tracing::warn!(%error, "cannot write lo ifindex into CONFIG");
    }

    let cgroup = match File::open(ROOT_CGROUP) {
        Ok(file) => file,
        Err(error) => {
            tracing::warn!(%error, "cannot open cgroup2 root; collector degraded");
            return (CollectorState::Failed, rx, None);
        }
    };

    // Attach cgroup_skb: try the non-intrusive AllowMultiple mode first, then
    // Single (validated in the spike at the root cgroup on this kernel —
    // AllowMultiple returns EINVAL there). Existing systemd firewall programs
    // are multi-attached and coexist; we never replace or detach them.
    for (label, attach_type) in [
        ("ingress", CgroupSkbAttachType::Ingress),
        ("egress", CgroupSkbAttachType::Egress),
    ] {
        let program: Option<&mut CgroupSkb> = bpf
            .program_mut(label)
            .and_then(|program| program.try_into().ok());
        let Some(program) = program else {
            tracing::warn!(program = label, "eBPF program missing from object");
            return (CollectorState::Failed, rx, None);
        };
        if let Err(error) = program.load() {
            tracing::warn!(program = label, %error, "eBPF program load failed");
            return (CollectorState::Failed, rx, None);
        }
        let attached = match program.attach(&cgroup, attach_type, CgroupAttachMode::AllowMultiple) {
            Ok(link) => {
                tracing::debug!(program = label, "attached with AllowMultiple");
                Some(link)
            }
            Err(multi_error) => {
                tracing::debug!(program = label, %multi_error, "AllowMultiple failed; trying Single");
                match program.attach(&cgroup, attach_type, CgroupAttachMode::Single) {
                    Ok(link) => Some(link),
                    Err(single_error) => {
                        tracing::warn!(program = label, %single_error, "eBPF attach failed; collector degraded");
                        return (CollectorState::Failed, rx, None);
                    }
                }
            }
        };
        // The link is held in Aya's ProgramData for the lifetime of `bpf`;
        // the returned ID carries no Drop and can be discarded safely.
        let _ = attached;
    }

    let Some(bytes_map) = bpf
        .take_map("BYTES")
        .and_then(|map| AyaHashMap::<MapData, u64, Counts>::try_from(map).ok())
    else {
        tracing::warn!("BYTES map missing or invalid; collector degraded");
        return (CollectorState::Failed, rx, None);
    };
    let stats_map = bpf
        .take_map("STATS")
        .and_then(|map| Array::<MapData, u64>::try_from(map).ok());
    let handle = tokio::spawn(async move {
        // Hold `bpf` for the collector's lifetime: dropping it detaches the
        // programs (the link fds live in ProgramData). The map wrappers moved
        // into the task keep the kernel maps open independently.
        let _bpf = bpf;
        sample_loop(bytes_map, stats_map, rates, tx).await;
    });
    (CollectorState::Attached, rx, Some(handle))
}

async fn sample_loop(
    map: AyaHashMap<MapData, u64, Counts>,
    stats: Option<Array<MapData, u64>>,
    rates: watch::Receiver<std::sync::Arc<RateSnapshot>>,
    tx: watch::Sender<std::sync::Arc<AppUsageFrame>>,
) {
    let mut previous: BTreeMap<u64, Counts> = BTreeMap::new();
    let mut tick = tokio::time::interval(SAMPLE_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut log_at = std::time::Instant::now();
    // Self-measured CPU/RSS from /proc (no sysstat dependency), reported in
    // the diagnostic log so load tests can check the ≤3% / ≤150 MB budget.
    let mut proc_prev = sample_proc();
    let mut cpu_pct = 0.0f64;
    let mut rss_kib = 0u64;
    loop {
        tick.tick().await;
        let mut current = BTreeMap::new();
        for item in map.iter().flatten() {
            current.insert(item.0, item.1);
        }
        let collector_stats =
            stats
                .as_ref()
                .map_or_else(CollectorStats::default, |s| CollectorStats {
                    bpf_packets_seen: s.get(&0, 0).unwrap_or(0),
                    bpf_loopback_skipped: s.get(&1, 0).unwrap_or(0),
                    bpf_map_full: s.get(&2, 0).unwrap_or(0),
                });
        // Periodic diagnostic (every 10 s) — proves the BPF program is writing
        // bytes into the map. Values are also published every tick for /status.
        if log_at.elapsed() >= Duration::from_secs(10) {
            let window_s = log_at.elapsed().as_secs_f64();
            let now_proc = sample_proc();
            if let (Some(prev), Some(now)) = (proc_prev.as_ref(), now_proc.as_ref()) {
                // Linux x86_64: USER_HZ=100; RSS pages are 4 KiB.
                let ticks = now.ticks.saturating_sub(prev.ticks);
                #[allow(clippy::cast_precision_loss)] // tick counts here are tiny
                let cpu_pct_value = (ticks as f64 / 100.0) / window_s.max(1.0) * 100.0;
                cpu_pct = cpu_pct_value;
                rss_kib = now.rss_pages.saturating_mul(4);
            }
            proc_prev = now_proc;
            tracing::info!(
                cgroup_entries = current.len(),
                packets = collector_stats.bpf_packets_seen,
                loopback = collector_stats.bpf_loopback_skipped,
                map_full = collector_stats.bpf_map_full,
                cpu_pct,
                rss_kib,
                "eBPF collector map diagnostic"
            );
            log_at = std::time::Instant::now();
        }

        // PC total from THIS tick's rate sample. `raw_*` is the unsmoothed
        // per-tick total, the correct base for the per-app sum check.
        let rate = rates.borrow().clone();
        let now = u64::try_from(crate::epoch_now()).unwrap_or(0);
        let mut rows = Vec::with_capacity(current.len());
        for (cgroup_id, counts) in &current {
            let prev = previous.get(cgroup_id).copied().unwrap_or_default();
            let down = counts.rx_bytes.saturating_sub(prev.rx_bytes);
            let up = counts.tx_bytes.saturating_sub(prev.tx_bytes);
            rows.push(AppUsage {
                cgroup_id: *cgroup_id,
                down_bps: down.saturating_mul(8),
                up_bps: up.saturating_mul(8),
                bytes_down: counts.rx_bytes,
                bytes_up: counts.tx_bytes,
                ts: now,
            });
        }
        previous = current;
        rows.sort_by_key(|row| std::cmp::Reverse(row.down_bps + row.up_bps));
        let _ = tx.send(std::sync::Arc::new(AppUsageFrame {
            ts: now,
            apps: rows,
            pc_down_bps: rate.raw_down_bps,
            pc_up_bps: rate.raw_up_bps,
            stats: collector_stats,
        }));
    }
}

/// /proc/self CPU ticks + RSS pages sample (avoids a sysstat dependency).
struct ProcSample {
    ticks: u64,
    rss_pages: u64,
}

fn sample_proc() -> Option<ProcSample> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // comm may contain spaces/parens; fields after the final ')' start at
    // state (field 3); utime/stime are fields 14/15 -> indexes 11/12.
    let rest = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let rss_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(ProcSample {
        ticks: utime.saturating_add(stime),
        rss_pages,
    })
}

fn lo_ifindex() -> Option<u32> {
    std::fs::read_to_string("/sys/class/net/lo/ifindex")
        .ok()?
        .trim()
        .parse()
        .ok()
}
