//! Net Watch Pro daemon — composition root (ARCHITECTURE.md §6.2).
//!
//! Phase 1' walking skeleton: config → Unix-socket API serving
//! `/health`, `/status`, `/stream` with a stub "monitoring" status.
//! Phase 2 in progress: DB open + forward-only migrations at boot (P2.4);
//! `ebpf_loader`, `collector`, `writer`, `pruner` tasks still to come.
//!
//! # Exit codes (§7.1 fail-fast startup classes)
//! 2 config · 3 token (retired, D1a) · 4 runtime · 5 server

// Tests are not daemon paths (ARCHITECTURE.md §7.1): unwrapping in test
// assertions is standard and expected.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod config;
mod db;
mod live;
mod sampler;

use netwatch_common::{DaemonState, EbpfState, StatusSnapshot};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, watch};

/// Startup failure classes, each with its §7.1 exit code.
enum StartupError {
    /// DB open/migration failed — runtime class.
    Db(db::DbError),
    /// API server failed — server class.
    Server(String),
}

fn main() {
    // Fail-fast startup (§7.1): env/config errors crash with a message;
    // runtime errors never do.
    let cfg = match config::Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("netwatchd: config error: {e}");
            std::process::exit(2);
        }
    };

    // NETWATCH_LOG_LEVEL (default info) via the standard env-filter syntax.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("netwatchd: cannot start async runtime: {e}");
            std::process::exit(4);
        }
    };

    match runtime.block_on(run(cfg)) {
        Ok(()) => {}
        Err(StartupError::Db(e)) => {
            eprintln!("netwatchd: database: {e}");
            std::process::exit(4);
        }
        Err(StartupError::Server(e)) => {
            eprintln!("netwatchd: {e}");
            std::process::exit(5);
        }
    }
}

/// Open the database, apply migrations, then serve until SIGINT/SIGTERM.
/// Status is a stub "monitoring" state (PLAN.md P1.6 explicitly allows the
/// stub until the Phase 2 collector).
async fn run(cfg: config::Config) -> Result<(), StartupError> {
    // P2.4: open SQLite and apply embedded migrations at boot (DATABASE.md
    // §8). Unwritable dir / newer schema → fail-fast with exit 4.
    let db_path = cfg.data_dir.join("netwatch.db");
    std::fs::create_dir_all(&cfg.data_dir)
        .map_err(|e| StartupError::Server(format!("cannot create data dir: {e}")))?;
    let pool = db::open(&db_path).await.map_err(StartupError::Db)?;
    db::migrate(&pool).await.map_err(StartupError::Db)?;
    tracing::info!(?db_path, "database open, migrations applied");

    // P4.1: compact raw episodes that already aged past the raw window on
    // boot. A failure is degraded state, never a startup failure (§7.1);
    // P7.1's scheduled pruner will own recurring runs.
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or_default();
    match db::compact_old_connections(&pool, now, netwatch_core::retention::DEFAULT_ENDPOINT_CAP)
        .await
    {
        Ok(report) if report.compacted_connections > 0 => tracing::info!(
            connections = report.compacted_connections,
            rollups = report.rollup_rows,
            endpoints = report.endpoint_rows,
            "raw-window compaction complete"
        ),
        Ok(_) => {}
        Err(e) => tracing::warn!("compaction failed; raw rows kept: {e}"),
    }

    // P4.2: interface counters use their own cadence; the first tick only
    // establishes the cumulative-counter baseline.
    let sampler_abort = sampler::spawn(pool.clone());

    // P2.5/P2.6 wiring: live service owns the episode store; the writer
    // task persists flush batches (≤ 1 tx/s); the API serves snapshot +
    // deltas. Event senders are held for the collector task (P2.1–P2.3,
    // gated on the eBPF spike) — until then nothing flows and the live
    // view is honestly empty.
    let start_id = db::max_connection_id(&pool)
        .await
        .map_err(StartupError::Db)?
        + 1;
    let (event_tx, event_rx) = mpsc::channel(1024);
    let (snapshot_tx, snapshot_rx) = watch::channel(Arc::new(Vec::new()));
    let (delta_tx, _) = broadcast::channel(256);
    let delta_tx = delta_tx.clone();
    let (flush_tx, mut flush_rx) = mpsc::channel::<Vec<netwatch_core::episode::EpisodeRow>>(64);
    let _events = live::LiveEventSender::new(event_tx); // collector takes this in P2.1–P2.3
    tokio::spawn(live::run(
        event_rx,
        live::LivePublishers {
            snapshot: snapshot_tx,
            deltas: delta_tx.clone(),
        },
        flush_tx,
        start_id,
    ));
    let writer_pool = pool.clone();
    tokio::spawn(async move {
        while let Some(batch) = flush_rx.recv().await {
            // Degraded, not dead (§7.1): a failed batch is logged and the
            // next checkpoint upserts absolute totals — no accumulation
            // error possible.
            if let Err(e) = db::persist_episodes(&writer_pool, &batch).await {
                tracing::warn!("episode flush failed ({} rows): {e}", batch.len());
            }
        }
        tracing::info!("writer task stopped");
    });

    let (status_tx, status_rx) = watch::channel(StatusSnapshot {
        daemon: DaemonState::Monitoring,
        ebpf: EbpfState::Attached,
        recording: true,
    });
    let _ = status_tx; // Phase 2 tasks will publish changes through this

    // Install signal handlers up front (fail-fast on handler errors).
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| StartupError::Server(format!("cannot install SIGTERM handler: {e}")))?;
    let shutdown = async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
        tracing::info!("shutdown signal received");
    };

    tracing::info!("serving API on {}", cfg.socket_path.display());
    let result = netwatch_api::serve_unix(
        &cfg.socket_path,
        status_rx,
        netwatch_api::LiveFeed {
            snapshot: snapshot_rx,
            deltas: delta_tx,
        },
        pool,
        shutdown,
    )
    .await
    .map_err(|e| StartupError::Server(format!("api server: {e}")));
    sampler_abort.abort();
    result
}
