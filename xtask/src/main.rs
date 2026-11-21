//! Build automation: BPF compile, packaging, release (ARCHITECTURE.md §4).
//! Verbs land per-phase: `bpf` (P2.1), `package` (P8.2), `release` (P8.8).

use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("bpf") => match bpf() {
            Ok(path) => println!("eBPF object: {}", path.display()),
            Err(error) => {
                eprintln!("xtask bpf failed: {error:#}");
                std::process::exit(1);
            }
        },
        Some("license-dev-key") => match license_dev_key() {
            Ok(()) => {}
            Err(error) => {
                eprintln!("xtask license-dev-key failed: {error:#}");
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!("usage: xtask bpf | xtask license-dev-key [expiry-unix-seconds]");
            std::process::exit(2); // distinct "no verb" exit, per fail-fast convention
        }
    }
}

/// Mint a fresh Ed25519 keypair and a signed NWP1 Pro key for owner testing.
///
/// The private half is printed once and never stored; give the public hex to a
/// debug daemon via `NETWATCH_LICENSE_PUBLIC_KEY`, then paste the key into the
/// Upgrade modal. `0` (default) means a perpetual key; any other value is a
/// Unix expiry second. Release binaries ignore the override (ARCHITECTURE §8.3).
fn license_dev_key() -> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    let expiry: u64 = std::env::args()
        .nth(2)
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(0);
    let mut seed = [0_u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut seed)?;
    let signing = SigningKey::from_bytes(&seed);

    let mut payload = vec![1_u8, 1]; // version 1, tier Pro (AUTH_SECURITY §2.4)
    payload.extend_from_slice(&expiry.to_be_bytes());
    payload.extend_from_slice(&[0_u8; 16]); // nonce
    let signature = signing.sign(&payload).to_bytes();
    let engine = base64::engine::general_purpose::STANDARD_NO_PAD;
    let key = format!(
        "NWP1-{}-{}",
        engine.encode(&payload),
        engine.encode(signature)
    );

    println!("DEV ONLY — do not use this key with a release build");
    println!(
        "public key hex: {}",
        hex(signing.verifying_key().as_bytes())
    );
    println!("license key: {key}");
    if expiry != 0 {
        println!("expires at unix second: {expiry}");
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// Build the separate eBPF crate and copy the object to `target/netwatch_ebpf.o`
/// for `netwatchd/build.rs` to embed (ARCHITECTURE.md §1.8).
///
/// Default: the active stable toolchain with `RUSTC_BOOTSTRAP=1` (measured to
/// work with 1.99.1 + rust-src + bpf-linker). `NWP_BPF_NIGHTLY=1` opts back
/// into a real nightly toolchain for toolchain debugging.
fn bpf() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo = repo_root()?;
    let crate_dir = repo.join("crates/netwatch-ebpf");
    let target_dir = crate_dir.join("target");
    let use_nightly = std::env::var_os("NWP_BPF_NIGHTLY").is_some();

    let args = [
        "build",
        "--release",
        "--target",
        "bpfel-unknown-none",
        "-Z",
        "build-std=core",
    ];
    let mut command = if use_nightly {
        let mut cmd = Command::new("rustup");
        cmd.args(["run", "nightly", "cargo"]);
        cmd
    } else {
        let mut cmd = Command::new("cargo");
        cmd.env("RUSTC_BOOTSTRAP", "1");
        cmd
    };
    // makepkg injects host RUSTFLAGS (e.g. -Ctarget-cpu=native). Those are
    // meaningless for the BPF target and make bpf-linker reject the host CPU
    // name ("tigerlake"), so strip them for this child process only.
    let status = command
        .args(args)
        .current_dir(&crate_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !status.success() {
        return Err(
            "eBPF build failed (need rust-src + bpf-linker; stable uses RUSTC_BOOTSTRAP=1)".into(),
        );
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
