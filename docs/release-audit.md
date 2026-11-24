# Release & Packaging Audit — Net Watch Pro

**Audited:** 2026-11-24  
**Auditor:** Prakash / Antigravity  
**Repository:** `net-watch-pro` (`https://github.com/zvirox/net-watch-pro`)

---

## 1. Workflows in `.github/workflows/`

* **`.github/workflows/ci.yml`**:
  * **Trigger:** `push` on `main`, `pull_request` on `main`.
  * **Jobs:** Format, clippy, unit & integration tests, frontend build.
* **`.github/workflows/hardening.yml`**:
  * **Trigger:** Push / PR / cron security scanning.
* **`.github/workflows/release.yml`**:
  * **Trigger:** `push` on tags matching `v*` or `workflow_dispatch`.
  * **Jobs:**
    1. `build-and-sign`: installs bpf-linker, builds eBPF collector via xtask, builds frontend and release binaries, runs custom `packaging/deb/build.sh` and `packaging/rpm/build.sh`, packs source and AUR tarballs, signs all assets with GPG key, uploads GitHub release.
    2. `verify-deb`: installs `.deb` in container, checks `netwatchd.service`, group membership, desktop file, uninstalls and verifies cleanup.

---

## 2. Existing Packaging Files & Configuration

* **`app/src-tauri/tauri.conf.json`**:
  * `"bundle": { "active": false, ... }` (Packaging handled entirely outside Tauri bundler)
* **`packaging/` directory contents:**
  * `packaging/deb/` (`build.sh`, `control`, `postinst`, `prerm`, `postrm`)
  * `packaging/rpm/` (`build.sh`, `net-watch-pro.spec`)
  * `packaging/arch/` (`PKGBUILD`, `.SRCINFO`, `net-watch-pro-git.install`)
  * `packaging/polkit/` (`pro.netwatch.enable-monitoring.policy`, `enable-monitoring`)
  * `packaging/tests/` (`test-clean-install.sh`, `test-uninstall.sh`, `test-upgrade.sh`)
  * `packaging/net-watch-pro.desktop`

---

## 3. Existing Tags and Release Assets

* **Tags:** `v0.0.1-alpha.1` up to `v0.9.2-beta.4`
* **Latest Release (`v0.9.2-beta.4`):**
  * `net-watch-pro_0.9.2-beta.4_amd64.deb` (6.90 MiB)
  * `net-watch-pro-0.9.2.beta.4-1.x86_64.rpm` (6.92 MiB)
  * `net-watch-pro-0.9.2-beta.4.tar.gz` (1.55 MiB)
  * `net-watch-pro-git-src.tar.gz` (3.12 KiB)
  * `SHA256SUMS` (493 B)
  * `.asc` detached GPG signatures for all files
  * `nwp-release-public.asc` (public GPG key)
  * *(Notice: No official standalone `.pkg.tar.zst` binary package was shipped; only an AUR source tarball)*

---

## 4. Version Alignment Check

| Location | Version | Matches Latest Tag (`v0.9.2-beta.4`)? |
|---|---|---|
| `Cargo.toml` (workspace) | `0.9.2-beta.4` | **Exact match** (`v0.9.2-beta.4`) |
| `app/package.json` | `0.9.2-beta.4` | **Exact match** |
| `app/src-tauri/tauri.conf.json` | `0.9.2-beta.4` | **Exact match** |

---

## 5. Naming Matrix & Identifier Alignment

* **GitHub Repository Name:** `net-watch-pro`
* **Cargo Binary Name:**
  * GUI App: `netwatch-app` (`app/src-tauri/Cargo.toml`)
  * Daemon: `netwatchd` (`crates/netwatchd/Cargo.toml`)
* **Installed Main Binary:** `/usr/bin/net-watch-pro` (The previous deb/rpm build scripts copied `netwatch-app` to `/usr/bin/net-watch-pro`).
* **Daemon Binary:** `/usr/bin/netwatchd`
* **Root Helper:** `/usr/libexec/netwatch-pro/enable-monitoring`
* **Tauri `productName`:** `"Net Watch Pro"`
* **Tauri Identifier:** `pro.netwatch.app`
* **Desktop File:** `packaging/net-watch-pro.desktop`
* **Service Unit:** `netwatchd.service`
* **Polkit Policy:** `pro.netwatch.enable-monitoring.policy`

---

## 6. Runtime Dependencies & Extra Files

* **Binaries & Scripts:**
  * `/usr/bin/net-watch-pro` (GUI desktop shell)
  * `/usr/bin/netwatchd` (eBPF background daemon)
  * `/usr/libexec/netwatch-pro/enable-monitoring` (Root onboarding helper)
* **Systemd Unit:** `/usr/lib/systemd/system/netwatchd.service`
* **Polkit Policy:** `/usr/share/polkit-1/actions/pro.netwatch.enable-monitoring.policy`
* **Desktop File:** `/usr/share/applications/net-watch-pro.desktop`
* **Icons:** `/usr/share/icons/hicolor/<size>/apps/net-watch-pro.png`
* **Postinst Tasks:**
  * Create system group `netwatch`.
  * Ensure runtime socket directory `/run/netwatch-pro` and config `/etc/netwatch-pro/allowed-uids`.
* **Runtime Dependencies:**
  * Debian/Ubuntu: `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`, `polkit-1`
  * Fedora/RPM: `webkit2gtk4.1`, `gtk3`, `libayatana-appindicator-gtk3`, `polkit`
  * Arch Linux: `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`, `polkit`

---

## 7. Secrets Used

* **`SIGNING_KEY`**, **`SIGNING_KEY_ID`**, **`SIGNING_PASSPHRASE`**: Used for GPG artifact signing.
* **`GITHUB_TOKEN`**: Repository write token.

---

## 8. Packaging Formats & Migration Action Table

| Format | Present | Job / File Location | Action |
|---|---|---|---|
| **Debian (.deb)** | Yes | `.github/workflows/release.yml` (`packaging/deb/build.sh`) | **Replace** with unified `nfpm` workflow |
| **RPM (.rpm)** | Yes | `.github/workflows/release.yml` (`packaging/rpm/build.sh`) | **Replace** with unified `nfpm` workflow |
| **Arch (.pkg.tar.zst)** | Partial | AUR source PKGBUILD only | **Replace** with unified `nfpm` binary package |
| **Tarball (.tar.gz)** | Yes | Source tarball & AUR tarball in release | **Delete** (uniform 3 packages only) |
| **Flatpak** | No | None | **N/A** |
| **AppImage** | No | None | **N/A** |
| **Checksums & GPG** | Yes | `SHA256SUMS` + detached signatures | **Keep** & standardize format |
