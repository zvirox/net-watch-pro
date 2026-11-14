//! Build automation: BPF compile, packaging, release (ARCHITECTURE.md §4).
//! Verbs land per-phase: `bpf` (P2.1), `package` (P8.2), `release` (P8.8).

fn main() {
    eprintln!("xtask skeleton — verbs land per PLAN.md phases");
    std::process::exit(2); // distinct "no verb" exit, per fail-fast convention
}
