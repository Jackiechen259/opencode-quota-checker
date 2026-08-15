# Updater migration (Iced → Tauri)

## Before: custom manifest updater

The archived Iced client implemented its own updater:

- `update/checker.rs` — fetched `update.json` (schema 1) from GitHub
  Releases, compared semver, resolved the current platform target.
- `update/download.rs` — streamed the package, verified SHA-256 against the
  manifest, landed it in the cache dir.
- `update/installer.rs` — launched NSIS (`/UPDATE`), opened DMG, replaced
  AppImage in place, opened deb with `xdg-open`.
- `xtask update-manifest` generated `update.json` + `SHA256SUMS` per release.

## After: Tauri updater

`tauri-plugin-updater` (v2) drives the whole flow from Rust:

- `check()` — fetches `latest.json`, resolves the current platform/target,
  applies the version comparator (stable channel only).
- `download(progress, done)` — downloads and **verifies the Ed25519
  signature** before returning the bytes (replaces the SHA-256 step).
- `install(bytes)` — writes the package and runs the platform installer:
  NSIS (Windows), DMG open (macOS), AppImage replace / deb (Linux).

The state machine stays in Rust (`AppState::updater`): Idle → Checking →
Available → Downloading → ReadyToInstall → Installing (+ UpToDate / Error).
The frontend only renders `update://state` snapshots, triggers commands, and
confirms the install. Auto-download when enabled mirrors the old behavior;
**install always requires explicit user confirmation**.

Signing key: the public key is baked into `src-tauri/tauri.conf.json`
(`plugins.updater.pubkey`); the private key + password live only in GitHub
Actions Secrets (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`).

## Iced → Tauri upgrade bridge

Installed Iced 0.1.2 clients check `update.json` (schema 1) and download the
NSIS / DMG / AppImage by the legacy filenames. The first Tauri release
therefore publishes **both**:

```text
latest.json + *.sig          → Tauri updater (future releases)
update.json + SHA256SUMS     → legacy Iced updater (one release cycle)
opencode-quota-checker-windows-x86_64.exe   (normalized Tauri NSIS copy)
...AppImage / ...deb / ...dmg               (normalized copies)
```

`prepare-legacy` in the release workflow normalizes the Tauri bundle names to
the legacy contract and regenerates `update.json` via
`cargo xtask update-manifest`, so the old client upgrades to the first Tauri
release, whose installer preserves config + keyring credential.

Keep the legacy manifest publishing for at least one release cycle after the
switch; afterwards only `latest.json` is needed.
