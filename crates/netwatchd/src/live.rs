//! Live-view service: episode store owner, 1 Hz sampler, delta publisher.
//!
//! ARCHITECTURE.md §6.2 task-per-concern: this task owns the in-memory
//! episode state (`netwatch_core::episode::EpisodeStore`), samples it once
//! per second (API.md §3: ≤ 1 msg/s per channel), publishes the snapshot
//! through a `watch` channel and deltas through a `broadcast` channel, and
//! hands flush batches to the writer task (P2.5).
//!
//! The collector (P2.1–P2.3, gated on the eBPF spike) will feed events into
//! `event_tx`; until then nothing flows and the live view is honestly empty.

use std::sync::Arc;
use std::time::Duration;

use netwatch_common::{LiveConnectionRow, LiveDelta};
use netwatch_core::episode::{EpisodeEvent, EpisodeStore, Sample};
use tokio::sync::{broadcast, mpsc, watch};

/// Publishers the live service feeds; the API layer consumes the receiving
/// halves (see `netwatch_api::LiveFeed`).
pub struct LivePublishers {
    /// Latest live snapshot (replaced once per sample).
    pub snapshot: watch::Sender<Arc<Vec<LiveConnectionRow>>>,
    /// Delta frames, broadcast to every subscribed WS client.
    pub deltas: broadcast::Sender<LiveDelta>,
}

/// Handle to feed collector events; `try_send` drops on a full bounded
/// queue (backpressure policy §10: bounded queues, never block the kernel
/// path; losses surface via diagnostics in a later task).
#[derive(Clone)]
pub struct LiveEventSender {
    tx: mpsc::Sender<EpisodeEvent>,
}

impl LiveEventSender {
    /// Wrap the events half of the collector channel.
    #[must_use]
    pub fn new(tx: mpsc::Sender<EpisodeEvent>) -> Self {
        Self { tx }
    }

    /// Enqueue an event; drops it when the bounded queue is full.
    pub fn send(&self, event: EpisodeEvent) {
        // Full queue: drop-oldest is not available on mpsc — drop newest
        // and let the next 1 Hz sample resync counters (they are absolute).
        let _ = self.tx.try_send(event);
    }
}

/// Cadences (F-09: checkpoint ~60 s; API.md §3: ≤ 1 msg/s).
const SAMPLE_TICK: Duration = Duration::from_secs(1);
const CHECKPOINT_EVERY_SAMPLES: u32 = 60;

/// Run the live service until `event_rx` closes (collector dropped) or the
/// process shuts down. Flush batches go to `flush_tx` (one tx per batch —
/// closes immediately, open episodes every ~60 s).
pub async fn run(
    event_rx: mpsc::Receiver<EpisodeEvent>,
    publishers: LivePublishers,
    flush_tx: mpsc::Sender<Vec<netwatch_core::episode::EpisodeRow>>,
    start_id: u64,
) {
    run_with_cadence(event_rx, publishers, flush_tx, start_id, SAMPLE_TICK).await;
}

/// `run` with an injectable sample cadence (tests shrink the 1 Hz tick).
async fn run_with_cadence(
    mut event_rx: mpsc::Receiver<EpisodeEvent>,
    publishers: LivePublishers,
    flush_tx: mpsc::Sender<Vec<netwatch_core::episode::EpisodeRow>>,
    start_id: u64,
    tick: Duration,
) {
    let mut store = EpisodeStore::new(start_id);
    let mut sample_tick = tokio::time::interval(tick);
    sample_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut sample_no: u32 = 0;

    // Initial empty snapshot so subscribers never wait for the first tick.
    publishers.snapshot.send_replace(Arc::new(Vec::new()));

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else {
                    // All event senders dropped: shut the service down.
                    break;
                };
                store.apply(event);
            }
            _ = sample_tick.tick() => {
                sample_no = sample_no.wrapping_add(1);
                let now = epoch_now();
                let Sample { delta, closed_rows } = store.sample(now);

                // Publish the fresh snapshot (watch: last value wins).
                publishers
                    .snapshot
                    .send_replace(Arc::new(store.snapshot()));

                // Broadcast the delta; no receivers yet is fine (Unbounded
                // lag handled per-connection on the API side).
                let _ = publishers.deltas.send(delta);

                // Closes persist immediately; open episodes on the ~60 s
                // checkpoint (F-09). One flush batch = one tx (P2.5).
                if !closed_rows.is_empty() || sample_no % CHECKPOINT_EVERY_SAMPLES == 0 {
                    let mut batch = closed_rows;
                    if sample_no % CHECKPOINT_EVERY_SAMPLES == 0 {
                        batch.extend(store.checkpoint_rows());
                    }
                    if !batch.is_empty() && flush_tx.send(batch).await.is_err() {
                        tracing::warn!("writer task gone; live flushes dropped");
                    }
                }
            }
        }
    }
    tracing::info!("live service stopped");
}

/// Epoch seconds (the store is clock-free; the service stamps events).
fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;
    use netwatch_common::Protocol;
    use netwatch_core::episode::EpisodeOpen;

    fn open_event(key: u64) -> EpisodeEvent {
        EpisodeEvent::Open(EpisodeOpen {
            key,
            app_id: 1,
            app_name: "Firefox".to_string(),
            icon: None,
            pid: 2213,
            process_name: Some("firefox".to_string()),
            protocol: Protocol::Tcp,
            local_ip: "192.168.1.23".to_string(),
            local_port: 53210,
            remote_ip: "151.101.1.1".to_string(),
            remote_port: 443,
            remote_host: None,
            interface_id: None,
            at: 1000,
        })
    }

    /// The service samples at the (shrunk) cadence: an Open event flows to
    /// the snapshot watch within one tick, and the close flush lands on the
    /// writer channel immediately after the next sample.
    #[tokio::test]
    async fn events_flow_to_snapshot_and_flush() {
        let (event_tx, event_rx) = mpsc::channel(8);
        let (snapshot_tx, mut snapshot_rx) =
            watch::channel(Arc::new(Vec::<LiveConnectionRow>::new()));
        let (delta_tx, mut delta_rx) = broadcast::channel(16);
        let (flush_tx, mut flush_rx) = mpsc::channel(8);

        let service = tokio::spawn(run_with_cadence(
            event_rx,
            LivePublishers {
                snapshot: snapshot_tx,
                deltas: delta_tx,
            },
            flush_tx,
            1,
            Duration::from_millis(20),
        ));

        LiveEventSender::new(event_tx.clone()).send(open_event(42));
        // Wait until the snapshot shows the episode (a few ticks).
        for _ in 0..50 {
            if !snapshot_rx.borrow().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let snap = snapshot_rx.borrow().clone();
        assert_eq!(snap.len(), 1, "episode appears in snapshot");
        assert_eq!(snap[0].app.name, "Firefox");

        // A delta was broadcast too (added row on first sample).
        let delta = delta_rx.recv().await.unwrap();
        assert_eq!(delta.added.len(), 1);

        // Close → next sample emits the removal and the final row flush.
        LiveEventSender::new(event_tx.clone()).send(EpisodeEvent::Close { key: 42, at: 1002 });
        let batch = tokio::time::timeout(Duration::from_secs(2), flush_rx.recv())
            .await
            .expect("close flush did not arrive")
            .expect("flush channel closed");
        assert_eq!(batch.len(), 1);
        assert!(!batch[0].is_open);

        drop(event_tx); // stop the service
        let _ = service.await.unwrap();
    }
}
