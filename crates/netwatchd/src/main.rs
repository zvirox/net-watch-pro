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

use netwatch_common::{DaemonState, EbpfState, StatusSnapshot};
use tokio::sync::watch;

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
    netwatch_api::serve_unix(&cfg.socket_path, status_rx, shutdown)
        .await
        .map_err(|e| StartupError::Server(format!("api server: {e}")))
}
