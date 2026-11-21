//! Phase C (C1–C3): eBPF collector for per-app byte attribution.
//!
//! Loads the separately built object, attaches `cgroup_skb` at the cgroup v2
//! root with the bounded capability set, and samples the per-cgroup byte map
//! once per second. Never panics: a missing object or failed attach is a
//! degraded state (ARCHITECTURE.md §7.1), not a startup failure.
//!
//! Identity comes from the socket's cgroup (`bpf_skb_cgroup_id` in the BPF
//! program), never from `current` — see docs/spike-ebpf-report.md §Twelfth.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aya::maps::{Array, HashMap as AyaHashMap, MapData};
use aya::programs::{CgroupAttachMode, CgroupSkb, CgroupSkbAttachType};
use aya::{Ebpf, Pod};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use netwatch_common::app_names::AppNames;
use netwatch_common::{AppUsage, AppUsageFrame, CollectorStats, Protocol, RateSnapshot};
use netwatch_core::episode::{EpisodeEvent, EpisodeOpen};
use sqlx::SqlitePool;

use crate::live::LiveEventSender;

/// Mirror of the BPF `Flow` struct in `crates/netwatch-ebpf/src/main.rs`.
/// Field order and widths must match exactly (the BPF map is raw bytes).
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct Flow {
    cgroup_id: u64,
    local_ip: [u32; 4],
    remote_ip: [u32; 4],
    local_port: u32,
    remote_port: u32,
    protocol: u32,
    _pad: u32,
    bytes_up: u64,
    bytes_down: u64,
    packets_up: u32,
    packets_down: u32,
    family: u32,
    _pad2: u32,
}
// SAFETY: repr(C) struct of integer fields with no padding-sensitive types.
#[allow(unsafe_code)]
unsafe impl Pod for Flow {}

/// A socket with no byte change for this long is closed in the live view.
/// No measured eBPF hook gives a reliable close callback (spike report §Twelfth).
const FLOW_IDLE_CLOSE_S: u64 = 120;

struct FlowEpisode {
    up: u64,
    down: u64,
    up_packets: u32,
    down_packets: u32,
    seen_at: u64,
}

fn ipv4(value: u32) -> String {
    std::net::Ipv4Addr::from(u32::from_be(value)).to_string()
}

fn format_ip(family: u32, words: [u32; 4]) -> String {
    if family == 2 {
        ipv4(words[0])
    } else if family == 10 {
        let mut octets = [0u8; 16];
        for (i, word) in words.iter().enumerate() {
            octets[i * 4..(i + 1) * 4].copy_from_slice(&word.to_ne_bytes());
        }
        std::net::Ipv6Addr::from(octets).to_string()
    } else {
        "—".to_string()
    }
}

fn protocol_from(value: u32) -> Protocol {
    match value {
        6 => Protocol::Tcp,
        17 => Protocol::Udp,
        _ => Protocol::Other,
    }
}

// The object is embedded at build time by build.rs (ARCHITECTURE.md §1.8).
// A plain checkout without `cargo xtask bpf` compiles with an empty slice and
// reports an honest degraded state; no runtime file path is needed.
#[cfg(nwp_embedded_ebpf)]
const EMBEDDED_OBJECT: &[u8] =
    aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/netwatch_ebpf.o"));
#[cfg(not(nwp_embedded_ebpf))]
const EMBEDDED_OBJECT: &[u8] = &[];

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

fn attach_programs(bpf: &mut Ebpf, cgroup: &File) -> bool {
    for (label, attach_type) in [
        ("ingress", CgroupSkbAttachType::Ingress),
        ("egress", CgroupSkbAttachType::Egress),
    ] {
        let program: Option<&mut CgroupSkb> = bpf
            .program_mut(label)
            .and_then(|program| program.try_into().ok());
        let Some(program) = program else {
            tracing::warn!(program = label, "eBPF program missing from object");
            return false;
        };
        if let Err(error) = program.load() {
            tracing::warn!(program = label, %error, "eBPF program load failed");
            return false;
        }
        let attached = match program.attach(cgroup, attach_type, CgroupAttachMode::AllowMultiple) {
            Ok(link) => {
                tracing::debug!(program = label, "attached with AllowMultiple");
                Some(link)
            }
            Err(multi_error) => {
                tracing::debug!(program = label, %multi_error, "AllowMultiple failed; trying Single");
                match program.attach(cgroup, attach_type, CgroupAttachMode::Single) {
                    Ok(link) => Some(link),
                    Err(single_error) => {
                        tracing::warn!(program = label, %single_error, "eBPF attach failed; collector degraded");
                        return false;
                    }
                }
            }
        };
        // The link is held in Aya's ProgramData for the lifetime of `bpf`;
        // the returned ID carries no Drop and can be discarded safely.
        let _ = attached;
    }
    true
}

/// Start the collector if enabled. Returns state + a watch receiver of the
/// latest per-app byte snapshot. Never returns an error for a missing object.
pub fn start(
    disabled: bool,
    object_override: Option<PathBuf>,
    rates: watch::Receiver<std::sync::Arc<RateSnapshot>>,
    pool: SqlitePool,
    names: Arc<AppNames>,
    episodes: Option<LiveEventSender>,
) -> (CollectorState, AppUsageRx, Option<JoinHandle<()>>) {
    let empty = std::sync::Arc::new(AppUsageFrame::default());
    let (tx, rx) = watch::channel(empty);
    if disabled {
        tracing::info!("NETWATCH_DISABLE_EBPF set: collector disabled");
        return (CollectorState::Disabled, rx, None);
    }
    // `NETWATCH_EBPF_OBJECT` is a dev override; packaged builds use the
    // embedded bytes and need no object file at runtime.
    let bytes: Cow<'static, [u8]> = if let Some(object) = object_override {
        match std::fs::read(&object) {
            Ok(bytes) => Cow::Owned(bytes),
            Err(error) => {
                tracing::warn!(path = %object.display(), %error, "eBPF object override unavailable; collector degraded");
                return (CollectorState::Disabled, rx, None);
            }
        }
    } else if EMBEDDED_OBJECT.is_empty() {
        tracing::warn!("no embedded eBPF object; collector degraded (run `cargo xtask bpf`)");
        return (CollectorState::Disabled, rx, None);
    } else {
        Cow::Borrowed(EMBEDDED_OBJECT)
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

    if !attach_programs(&mut bpf, &cgroup) {
        return (CollectorState::Failed, rx, None);
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
    // FLOWS is additive: an older dev object without it still gives per-app
    // rollups, just no connection rows.
    let flows_map = bpf
        .take_map("FLOWS")
        .and_then(|map| AyaHashMap::<MapData, u64, Flow>::try_from(map).ok());
    if flows_map.is_none() {
        tracing::warn!("FLOWS map missing; live connections will stay empty");
    }
    let handle = tokio::spawn(async move {
        // Hold `bpf` for the collector's lifetime: dropping it detaches the
        // programs (the link fds live in ProgramData). The map wrappers moved
        // into the task keep the kernel maps open independently.
        let _bpf = bpf;
        sample_loop(
            bytes_map,
            stats_map,
            FlowSources {
                map: flows_map,
                pool,
                names,
                episodes,
            },
            rates,
            tx,
        )
        .await;
    });
    (CollectorState::Attached, rx, Some(handle))
}

/// Byte delta over a measured interval → decimal bits per second.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped finite, non-negative
fn bytes_to_bps(delta_bytes: u64, elapsed_s: f64) -> u64 {
    let bps = netwatch_core::rates::bits_per_second(delta_bytes, elapsed_s);
    if bps.is_finite() && bps > 0.0 {
        bps.round().min(1.0e18) as u64
    } else {
        0
    }
}

struct FlowSources {
    map: Option<AyaHashMap<MapData, u64, Flow>>,
    pool: SqlitePool,
    names: Arc<AppNames>,
    episodes: Option<LiveEventSender>,
}

async fn sample_loop(
    map: AyaHashMap<MapData, u64, Counts>,
    stats: Option<Array<MapData, u64>>,
    sources: FlowSources,
    rates: watch::Receiver<std::sync::Arc<RateSnapshot>>,
    tx: watch::Sender<std::sync::Arc<AppUsageFrame>>,
) {
    let mut previous: BTreeMap<u64, Counts> = BTreeMap::new();
    // Instant of the previous map read; `None` until the baseline tick.
    let mut last_read: Option<std::time::Instant> = None;
    let mut flow_state: BTreeMap<u64, FlowEpisode> = BTreeMap::new();
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
        // Rates use the measured interval, not an assumed 1 s: a delayed tick
        // (MissedTickBehavior::Delay, or slow DB writes in `process_flows`)
        // must not inflate per-app speeds. The first read is only a baseline.
        let read_at = std::time::Instant::now();
        let elapsed_s = last_read.map(|at| read_at.duration_since(at).as_secs_f64());
        last_read = Some(read_at);
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
        if let (Some(flow_map), Some(sender)) = (sources.map.as_ref(), sources.episodes.as_ref()) {
            process_flows(
                flow_map,
                &mut flow_state,
                &sources.pool,
                sources.names.as_ref(),
                sender,
                now,
            )
            .await;
        }
        let mut rows = Vec::with_capacity(current.len());
        for (cgroup_id, counts) in &current {
            let prev = previous.get(cgroup_id).copied().unwrap_or_default();
            let down = counts.rx_bytes.saturating_sub(prev.rx_bytes);
            let up = counts.tx_bytes.saturating_sub(prev.tx_bytes);
            let (down_bps, up_bps) = elapsed_s.map_or((0, 0), |elapsed| {
                (bytes_to_bps(down, elapsed), bytes_to_bps(up, elapsed))
            });
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
        let _ = tx.send(std::sync::Arc::new(AppUsageFrame {
            ts: now,
            apps: rows,
            pc_down_bps: rate.raw_down_bps,
            pc_up_bps: rate.raw_up_bps,
            stats: collector_stats,
        }));
    }
}

/// Turn the BPF per-socket flow map into episode events for the live service.
///
/// `Bytes` values are absolute counters (the episode store clamps regressions),
/// and a cookie with no byte change for `FLOW_IDLE_CLOSE_S` is closed. No PID
/// is invented: `pid_hint` is best-effort from the cgroup scope name and 0
/// renders as "—" in the UI.
async fn process_flows(
    flows: &AyaHashMap<MapData, u64, Flow>,
    state: &mut BTreeMap<u64, FlowEpisode>,
    pool: &SqlitePool,
    names: &AppNames,
    episodes: &LiveEventSender,
    now: u64,
) {
    let now_i64 = i64::try_from(now).unwrap_or(0);
    let mut current = BTreeMap::new();
    for item in flows.iter().flatten() {
        current.insert(item.0, item.1);
    }

    for (cookie, flow) in &current {
        if flow.family != 2 || flow.cgroup_id == 0 {
            continue; // IPv6/unattributed bytes stay in the cgroup rollups
        }
        let known = state.contains_key(cookie);
        if !known {
            let resolved = names.resolve(flow.cgroup_id, 0, 0);
            let (app_id, app_name, _confidence) = match crate::db::ensure_cgroup_app(
                pool,
                flow.cgroup_id,
                &resolved.name,
                &resolved.confidence,
                now_i64,
            )
            .await
            {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(%error, cgroup_id = flow.cgroup_id, "cannot register flow app");
                    continue;
                }
            };
            episodes.send(EpisodeEvent::Open(EpisodeOpen {
                key: *cookie,
                app_id,
                app_name: app_name.clone(),
                icon: None,
                pid: names.pid_hint(flow.cgroup_id),
                process_name: None,
                protocol: protocol_from(flow.protocol),
                local_ip: format_ip(flow.family, flow.local_ip),
                local_port: u16::try_from(flow.local_port).unwrap_or(0),
                remote_ip: format_ip(flow.family, flow.remote_ip),
                remote_port: u16::try_from(flow.remote_port).unwrap_or(0),
                remote_host: None,
                interface_id: None,
                at: now,
            }));
            state.insert(
                *cookie,
                FlowEpisode {
                    up: flow.bytes_up,
                    down: flow.bytes_down,
                    up_packets: flow.packets_up,
                    down_packets: flow.packets_down,
                    seen_at: now,
                },
            );
            continue; // first Bytes arrive on the next tick
        }
        let Some(entry) = state.get_mut(cookie) else {
            continue;
        };
        if flow.bytes_up != entry.up
            || flow.bytes_down != entry.down
            || flow.packets_up != entry.up_packets
            || flow.packets_down != entry.down_packets
        {
            episodes.send(EpisodeEvent::Bytes {
                key: *cookie,
                bytes_up: flow.bytes_up,
                bytes_down: flow.bytes_down,
                packets_up: u64::from(flow.packets_up),
                packets_down: u64::from(flow.packets_down),
                at: now,
            });
            entry.up = flow.bytes_up;
            entry.down = flow.bytes_down;
            entry.up_packets = flow.packets_up;
            entry.down_packets = flow.packets_down;
            entry.seen_at = now;
        }
    }

    let closed_keys: Vec<u64> = state
        .iter()
        .filter(|(key, entry)| {
            !current.contains_key(key) || now.saturating_sub(entry.seen_at) > FLOW_IDLE_CLOSE_S
        })
        .map(|(key, _)| *key)
        .collect();
    for key in closed_keys {
        episodes.send(EpisodeEvent::Close { key, at: now });
        state.remove(&key);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_app_rate_uses_measured_interval() {
        // 1,000,000 bytes over 1 s = 8 Mbit/s; the same bytes over a 2 s
        // delayed tick are 4 Mbit/s, not 8 (the old fixed-1 s assumption).
        assert_eq!(bytes_to_bps(1_000_000, 1.0), 8_000_000);
        assert_eq!(bytes_to_bps(1_000_000, 2.0), 4_000_000);
        assert_eq!(
            bytes_to_bps(1_000_000, 0.0),
            0,
            "zero interval is not infinite"
        );
        assert_eq!(bytes_to_bps(0, 1.0), 0);
    }

    #[test]
    fn flow_layout_matches_bpf_struct() {
        assert_eq!(std::mem::size_of::<Flow>(), 88);
    }

    #[test]
    fn ipv4_converts_network_byte_order() {
        // 151.101.1.1 stored as a network-order u32 on a little-endian host.
        assert_eq!(ipv4(0x0101_6597), "151.101.1.1");
    }

    #[test]
    fn ipv6_formats_canonical_address() {
        // 2001:db8::1 in network bytes: 20 01 0d b8, 00 00 00 00, 00 00 00 00, 00 00 00 01
        let words = [
            u32::from_ne_bytes([0x20, 0x01, 0x0d, 0xb8]),
            u32::from_ne_bytes([0x00, 0x00, 0x00, 0x00]),
            u32::from_ne_bytes([0x00, 0x00, 0x00, 0x00]),
            u32::from_ne_bytes([0x00, 0x00, 0x00, 0x01]),
        ];
        assert_eq!(format_ip(10, words), "2001:db8::1");
    }

    #[test]
    fn protocols_map_to_contract_values() {
        assert_eq!(protocol_from(6), Protocol::Tcp);
        assert_eq!(protocol_from(17), Protocol::Udp);
        assert_eq!(protocol_from(1), Protocol::Other);
    }

    /// build.rs embeds the object only when `cargo xtask bpf` ran first. In
    /// that case the bytes must be a valid ELF64 eBPF object.
    #[test]
    fn embedded_object_is_elf_when_present() {
        if cfg!(nwp_embedded_ebpf) {
            assert!(EMBEDDED_OBJECT.starts_with(&[0x7f, b'E', b'L', b'F']));
            assert!(EMBEDDED_OBJECT.len() > 64);
        } else {
            assert_eq!(EMBEDDED_OBJECT.len(), 0, "unexpected embedded bytes");
        }
    }
}
