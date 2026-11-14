//! Net Watch Pro daemon — composition root.
//!
//! ARCHITECTURE.md §6.2: task-per-concern (ebpf-loader, collector, writer,
//! pruner, api, health) lands across Phase 1–2. This stub keeps the binary
//! runnable so every phase ends in a runnable state (PLAN.md §14).

fn main() {
    // Fail-fast placeholder: real startup (config, token file, API bind)
    // arrives with task P1.1 (PLAN.md §3).
    eprintln!("netwatchd skeleton — Phase 0 (P0.2); real boot lands in P1.1");
    std::process::exit(0);
}
