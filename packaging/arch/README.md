# AUR dev-channel package (P8.4)

`net-watch-pro-git` builds the current `main` branch from GitHub. Stable
`net-watch-pro` / `net-watch-pro-bin` packages are planned for GA (P11.1).

## Files

| File | Purpose |
|---|---|
| `PKGBUILD` | AUR recipe: git source, release build, file layout |
| `net-watch-pro-git.install` | Root-side hooks: socket group, GID drop-in, state dir, cleanup |
| `.SRCINFO` | Generated metadata required by the AUR web interface |

The hardened unit comes from `packaging/systemd/`, the desktop entry from
`packaging/deb/` — no duplicated copies.

## Build locally

```bash
cd packaging/arch
makepkg -s             # builds net-watch-pro-git-*.pkg.tar.zst
makepkg --printsrcinfo > .SRCINFO   # after any PKGBUILD change
```

Install with `paru -S net-watch-pro-git` from AUR. The package leaves
`netwatchd.service` **disabled**; onboarding (P8.5) enables it after health
checks. The `.install` hook creates the `netwatch` group used by the `0660`
API socket (see `ARCHITECTURE.md` §8.3).

Removal cleans the unit drop-in and allowed-UID list and **deletes
`/var/lib/netwatch-pro` by default**. To keep recorded data, create
`/etc/netwatch-pro/keep-data` before uninstalling (PRD opt-in retention, P8.7).

## Verification note

`makepkg --printsrcinfo` parses the PKGBUILD and the `package()` file layout was
exercised locally with staged release binaries. A full `makepkg -s` build on a
clean Arch/CachyOS VM (and the `paru` install path) is still pending.
