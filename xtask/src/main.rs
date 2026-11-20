//! Build automation: BPF compile, packaging, release (ARCHITECTURE.md §4).
//! Verbs land per-phase: `bpf` (P2.1), `package` (P8.2), `release` (P8.8).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("bpf") {
        match bpf() {
            Ok(path) => println!("eBPF object: {}", path.display()),
            Err(error) => {
                eprintln!("xtask bpf failed: {error:#}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("xtask skeleton — verbs land per PLAN.md phases (current: bpf)");
        std::process::exit(2); // distinct "no verb" exit, per fail-fast convention
    }
}

/// Build the separate eBPF crate with nightly + -Z build-std=core and copy
/// the object to the path the daemon loads by default (ARCHITECTURE.md §8.3).
fn bpf() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo = repo_root()?;
    let crate_dir = repo.join("crates/netwatch-ebpf");
    let target_dir = crate_dir.join("target");

    let status = Command::new("rustup")
        .args([
            "run",
            "nightly",
            "cargo",
            "build",
            "--release",
            "--target",
            "bpfel-unknown-none",
            "-Z",
            "build-std=core",
        ])
        .current_dir(&crate_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !status.success() {
        return Err("nightly eBPF build failed (need nightly + rust-src + bpf-linker)".into());
    }

    let src = target_dir.join("bpfel-unknown-none/release/netwatch-ebpf");
    let out_dir = repo.join("target");
    std::fs::create_dir_all(&out_dir)?;
    let dst = out_dir.join("netwatch_ebpf.o");
    std::fs::copy(&src, &dst)?;
    Ok(dst)
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut dir = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("crates").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("could not locate repository root".into());
        }
    }
}
