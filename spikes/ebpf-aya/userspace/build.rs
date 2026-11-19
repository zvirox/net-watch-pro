fn main() -> anyhow::Result<()> {
    // Builds the sibling `ebpf` crate for bpfel-unknown-none with nightly +
    // bpf-linker and places the object in OUT_DIR. `AYA_BUILD_SKIP=1` is
    // supported by aya-build for tooling that only needs the host binary.
    aya_build::build_ebpf(
        [aya_build::Package {
            name: "netwatch-spike-ebpf",
            root_dir: "../",
            ..Default::default()
        }],
        aya_build::Toolchain::default(),
    )
}
