# Release workflow (P8.8)

`release.yml` runs on any `v*` tag (or manual dispatch with a `tag` input). It:

1. Builds the UI and release binaries with the pinned Rust/Node toolchains.
2. Builds the `.deb`, `.rpm`, source tarball and AUR source tarball.
3. Signs every artifact with detached armored GPG signatures and signs
   `SHA256SUMS` as well, then verifies all signatures.
4. Publishes the assets on a GitHub Release (pre-release unless the tag is a
   plain `vX.Y.Z` stable release).
5. Installs the `.deb` on a clean Ubuntu runner, checks the unit/desktop entry
   and the opt-in uninstall contract (P8.7).

Repository admins must configure these **Actions secrets** (never commit the
private key):

| Secret | Purpose |
|---|---|
| `SIGNING_KEY` | Armored ASCII private GPG key (`gpg --armor --export-secret-keys …`) |
| `SIGNING_KEY_ID` | Key ID/fingerprint used with `--local-user` |
| `SIGNING_PASSPHRASE` | Passphrase for the signing key |

Suggested generation (offline, then store the three values in GitHub Settings →
Secrets → Actions):

```bash
gpg --quick-generate-key 'Net Watch Pro Release <releases@netwatchpro.invalid>' ed25519 sign never
gpg --armor --export-secret-keys 'Net Watch Pro Release'   # -> SIGNING_KEY
gpg --list-secret-keys --keyid-format=long                  # -> SIGNING_KEY_ID
```

Publish the corresponding **public** key on the releases page so users can run
`gpg --verify net-watch-pro_*.deb.asc`. The private key is never part of the
repository or the release assets.

## Packaging channels

Alpha/beta ship GitHub Release artifacts plus AUR `net-watch-pro-git`; hosted
apt/dnf repositories and stable AUR `-`/`-bin` packages are deferred until
after beta (PLAN F-10 / P11.1). First installable tag: `v0.8.0-alpha.8`.
