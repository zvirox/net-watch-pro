//! Embed the prebuilt eBPF object into `netwatchd` (ARCHITECTURE.md §1.8).
//!
//! `cargo xtask bpf` writes `target/netwatch_ebpf.o`; this script copies it
//! into `OUT_DIR` so `aya::include_bytes_aligned!` can embed it. The object is
//! never committed and is not needed at runtime. When it is absent (plain
//! `cargo build` in a fresh checkout) the daemon compiles without embedded
//! bytes and reports an honest `no_collector` state; CI and the AUR recipe
//! must build it before compiling this crate.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(nwp_embedded_ebpf)");

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let repo = manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(manifest);
    let source = repo.join("target").join("netwatch_ebpf.o");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap_or_default());

    if source.is_file() {
        let destination = out.join("netwatch_ebpf.o");
        if fs::copy(&source, &destination).is_ok() {
            println!("cargo:rustc-cfg=nwp_embedded_ebpf");
            println!("cargo:rerun-if-changed={}", source.display());
        } else {
            println!(
                "cargo:warning=could not copy eBPF object; daemon will run without an embedded collector"
            );
        }
    } else {
        println!(
            "cargo:warning=no target/netwatch_ebpf.o; run `cargo xtask bpf` to embed a collector"
        );
    }
}
