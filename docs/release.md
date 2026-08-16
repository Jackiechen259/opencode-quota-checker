# Release guide

The root `Cargo.toml` `[workspace.package].version` is the authoritative
version. `cargo xtask` mirrors it into `package.json` and
`src-tauri/tauri.conf.json`. `src-tauri/Cargo.toml` inherits the workspace
version automatically.

Prepare a release from a clean working tree:

```bash
cargo xtask release patch
cargo xtask release minor
cargo xtask release major
cargo xtask release 1.0.0-rc.1
```

The command updates every version metadata file and `Cargo.lock`, formats,
lints, tests, commits, and creates an annotated `vVERSION` tag. It does not
push by default. Add `--push` to push the current branch and tag.
`cargo xtask verify-version [vVERSION]` checks all three files (and the tag)
stay in sync and is part of CI.

A pushed `v*` tag runs `.github/workflows/release.yml`:

```text
resolve
   ↓
package (Windows x64 NSIS / Linux x64 deb+AppImage / macOS ARM dmg)
   │     └ tauri-action: bundles + updater signatures + merged latest.json,
   │       published to a draft GitHub release
   ↓
prepare-legacy (normalize legacy asset names, SHA256SUMS, update.json)
   ↓
finalize (un-draft the release)
```

## Packaging

`package` jobs build through `tauri-action` with updater signing enabled:

- `TAURI_SIGNING_PRIVATE_KEY` — the private key contents (secret)

The current key is passwordless (generated with `cargo tauri signer generate -p ""`),
so `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` is not needed; if the key ever has a
password again, it must be provided as a second secret. The private key lives
only in GitHub Actions Secrets. The public key is in
`src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

Local packaging uses the Tauri bundler:

```bash
pnpm tauri build --bundles nsis        # Windows per-user NSIS
pnpm tauri build --bundles deb,appimage
pnpm tauri build --bundles dmg
```

## Release assets

Tauri bundles keep their default names (e.g. `OpenCode Quota Checker_0.1.2_x64-setup.exe`,
`opencode-quota-checker_0.1.2_amd64.deb`, `..._aarch64.dmg`). The
`prepare-legacy` job additionally attaches the legacy updater bridge with the
exact names the old Iced client (0.1.2) downloads:

```text
opencode-quota-checker-windows-x86_64.exe
opencode-quota-checker-linux-x86_64.AppImage
opencode-quota-checker-linux-x86_64.deb
opencode-quota-checker-macos-aarch64.dmg
```

Every release additionally attaches:

- `SHA256SUMS` — one `<sha256>  <filename>` line per asset, sorted by name.
- `update.json` — the legacy Iced auto-update manifest (schema 1).
- `latest.json` + per-bundle `.sig` files — the Tauri updater artifacts,
  published by `tauri-action`.

macOS Intel is not built or published.

## Legacy update.json (Iced → Tauri bridge)

`prepare-legacy` runs `cargo xtask update-manifest <tag> release-assets`, which
validates the tag, verifies that every required platform asset exists,
computes SHA-256 digests, and writes `SHA256SUMS` and `update.json`. Missing
required assets fail the job.

The bridge exists so installed Iced 0.1.2 clients discover the first Tauri
release through their built-in updater and install the Tauri NSIS/DMG/AppImage
over their current version (config and keyring credential survive, as both use
the same paths/entries). Keep publishing the legacy manifest for at least one
release cycle after the switch.

## Updater verification

After publishing, verify the release page contains all platform bundles,
signatures, `latest.json`, `update.json`, and `SHA256SUMS`, and that the
updater manifests are reachable at:

```text
https://github.com/Jackiechen259/opencode-quota-checker/releases/latest/download/latest.json
https://github.com/Jackiechen259/opencode-quota-checker/releases/latest/download/update.json
```

Then install the previous release and confirm the updater detects the new
version, downloads, verifies the signature, and offers to install it after
explicit user confirmation. Check each platform's install path (NSIS, DMG,
AppImage, deb) per the release smoke test.

## Code signing

The updater packages are Ed25519-signed (Tauri updater), which is independent
of platform code signing. The platform installers themselves remain unsigned
until Windows Authenticode / macOS Developer ID credentials are configured;
SmartScreen and Gatekeeper may warn on first run. Never commit a private key,
`.pfx` file, or password.
