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

use netwatch_common::AppUsage;

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
pub type AppUsageRx = watch::Receiver<std::sync::Arc<Vec<AppUsage>>>;

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
) -> (CollectorState, AppUsageRx, Option<JoinHandle<()>>) {
    let (tx, rx) = watch::channel(std::sync::Arc::new(Vec::new()));
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
        sample_loop(bytes_map, stats_map, tx).await;
    });
    (CollectorState::Attached, rx, Some(handle))
}

async fn sample_loop(
    map: AyaHashMap<MapData, u64, Counts>,
    _stats: Option<Array<MapData, u64>>,
    tx: watch::Sender<std::sync::Arc<Vec<AppUsage>>>,
) {
    let mut previous: BTreeMap<u64, Counts> = BTreeMap::new();
    let mut tick = tokio::time::interval(SAMPLE_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        let mut current = BTreeMap::new();
        for item in map.iter().flatten() {
            current.insert(item.0, item.1);
        }
        let now = u64::try_from(crate::epoch_now()).unwrap_or(0);
        let mut rows = Vec::with_capacity(current.len());
        for (cgroup_id, counts) in &current {
            let prev = previous.get(cgroup_id).copied().unwrap_or_default();
            let rx = counts.rx_bytes.saturating_sub(prev.rx_bytes);
            let tx = counts.tx_bytes.saturating_sub(prev.tx_bytes);
            let down_bps = rx.saturating_mul(8); // per 1 s interval, decimal bits/s
            let up_bps = tx.saturating_mul(8);
            rows.push(AppUsage {
                cgroup_id: *cgroup_id,
                down_bps,
                up_bps,
                bytes_down: counts.rx_bytes,
                bytes_up: counts.tx_bytes,
                ts: now,
            });
        }
        previous = current;
        rows.sort_by_key(|row| std::cmp::Reverse(row.down_bps + row.up_bps));
        let _ = tx.send(std::sync::Arc::new(rows));
    }
}

fn lo_ifindex() -> Option<u32> {
    std::fs::read_to_string("/sys/class/net/lo/ifindex")
        .ok()?
        .trim()
        .parse()
        .ok()
}
