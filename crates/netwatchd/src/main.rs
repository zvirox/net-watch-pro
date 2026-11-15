//! Net Watch Pro daemon — composition root (ARCHITECTURE.md §6.2).
//!
//! Phase 1 walking skeleton: config → per-boot token → loopback API serving
//! `/health`, `/status`, `/stream` with a stub "monitoring" status. The
//! task-per-concern set (`ebpf_loader`, `collector`, `writer`, `pruner`) lands
//! across Phase 2+; each will own one Tokio task communicating over bounded
//! channels (§6.2/§10 backpressure).
//!
//! # Exit codes (§7.1 fail-fast startup classes)
//!
//! 2 config · 3 token (RNG or file) · 4 runtime · 5 server

// Tests are not daemon paths (ARCHITECTURE.md §7.1): unwrapping in test
// assertions is standard and expected.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod config;
mod token;

use netwatch_common::{DaemonState, EbpfState, StatusSnapshot};
use tokio::sync::watch;

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

    let token = match token::generate() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("netwatchd: cannot generate per-boot token: {e}");
            std::process::exit(3);
        }
    };
    if let Err(e) = token::write_token_file(&cfg.token_file, &token) {
        eprintln!(
            "netwatchd: cannot write token file {}: {e}",
            cfg.token_file.display()
        );
        std::process::exit(3);
    }
    tracing::info!("per-boot token written to {}", cfg.token_file.display());
    tracing::debug!("data dir: {}", cfg.data_dir.display());

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

    if let Err(e) = runtime.block_on(run(cfg, token)) {
        eprintln!("netwatchd: {e}");
        std::process::exit(5);
    }
}

/// Serve until SIGINT/SIGTERM. Status is a stub "monitoring/attached" state
/// (PLAN.md P1.6 explicitly allows the stub until the Phase 2 collector).
async fn run(cfg: config::Config, token: String) -> Result<(), String> {
    let (status_tx, status_rx) = watch::channel(StatusSnapshot {
        daemon: DaemonState::Monitoring,
        ebpf: EbpfState::Attached,
        recording: true,
    });
    let _ = status_tx; // Phase 2 tasks will publish changes through this

    // Install signal handlers up front (fail-fast on handler errors).
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| format!("cannot install SIGTERM handler: {e}"))?;
    let shutdown = async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
        tracing::info!("shutdown signal received");
    };

    tracing::info!("serving API on {}", cfg.api_addr);
    netwatch_api::serve(cfg.api_addr, token.into(), status_rx, shutdown)
        .await
        .map_err(|e| format!("api server: {e}"))
}
