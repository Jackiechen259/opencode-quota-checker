# Release guide

`[workspace.package].version` in the root `Cargo.toml` is the authoritative
version. `cargo xtask` mirrors it into `packager.json` because cargo-packager
requires an explicit package version.

Prepare a release from a clean working tree:

```bash
cargo xtask release patch
cargo xtask release minor
cargo xtask release major
cargo xtask release 1.0.0-rc.1
```

The command updates version metadata and `Cargo.lock`, formats, lints, tests,
commits, and creates an annotated `vVERSION` tag. It does not push by default.
Add `--push` to push the current branch and tag.

A pushed `v*` tag runs `.github/workflows/release.yml`:

```text
resolve
   ↓
package (Windows x64 / Linux x64 / macOS Apple Silicon)
   ↓
prepare-release (normalize assets, SHA256SUMS, update.json)
   ↓
publish (GitHub release)
```

The publish job only runs when every package job and the manifest generation
succeeded, so a release can never point at a missing asset. Tags containing a
prerelease suffix are marked as prereleases and are ignored by the stable
update channel.

## Release assets

Packages are renamed to stable names before upload, and the updater relies on
exactly these filenames:

```text
opencode-quota-checker-windows-x86_64.exe
opencode-quota-checker-linux-x86_64.AppImage
opencode-quota-checker-linux-x86_64.deb
opencode-quota-checker-macos-aarch64.dmg
```

Every release additionally attaches:

- `SHA256SUMS` — one `<sha256>  <filename>` line per asset, sorted by name.
- `update.json` — the auto-update manifest (see below).

macOS Intel is not built or published.

## update.json

`prepare-release` runs `cargo xtask update-manifest <tag> release-assets`,
which validates the tag, verifies that every required platform asset exists,
computes SHA-256 digests, and writes `SHA256SUMS` and `update.json` into the
asset directory. Missing required assets fail the job.

```json
{
  "schema": 1,
  "version": "0.2.0",
  "tag": "v0.2.0",
  "prerelease": false,
  "release_notes_url": "https://github.com/Jackiechen259/opencode-quota-checker/releases/tag/v0.2.0",
  "platforms": {
    "windows-x86_64": {
      "type": "nsis",
      "url": "https://github.com/Jackiechen259/opencode-quota-checker/releases/download/v0.2.0/opencode-quota-checker-windows-x86_64.exe",
      "sha256": "..."
    },
    "linux-x86_64-appimage": { "type": "appimage", "url": "...", "sha256": "..." },
    "linux-x86_64-deb": { "type": "deb", "url": "...", "sha256": "..." },
    "macos-aarch64": { "type": "dmg", "url": "...", "sha256": "..." }
  }
}
```

The manifest can be generated and inspected locally:

```bash
cargo xtask update-manifest v0.2.0 release-assets
```

## Windows installer

Windows users download a single per-user NSIS setup executable:

```text
opencode-quota-checker-windows-x86_64.exe
```

Installing it (no administrator rights required) places the application under
`%LOCALAPPDATA%\OpenCode Quota Checker`, registers it in **Settings → Installed
apps**, and adds an **OpenCode Quota Checker** Start Menu entry. Running a newer
installer over an older release upgrades it in place. User configuration
(`%APPDATA%\opencode-quota-checker`), cached update downloads
(`%LOCALAPPDATA%\...\opencode-quota-checker\update`), and the keyring credential
survive both upgrades and uninstall; uninstalling removes only the installed
program and shortcuts.

The same executable is the package the built-in auto-updater downloads for
Windows, so every release must keep the exact filename above and keep the
`update.json` `windows-x86_64` → `nsis` entry in sync. `cargo xtask
update-manifest` and `verify-version` fail the build on any drift.

Installers are unsigned until code signing is configured (below); Windows
SmartScreen may warn on first run.

## Code signing

Packages are currently unsigned. Releases work without a certificate, but
Windows SmartScreen shows a warning for unsigned downloads and macOS Gatekeeper
requires extra steps for unsigned packages.

Add an optional signing stage that only runs when signing credentials are
configured, so unsigned builds keep working. Never commit a private key, `.pfx`
file, or password.

For Windows Authenticode signing, store the base64-encoded certificate and its
password as repository secrets:

```text
WINDOWS_CERTIFICATE
WINDOWS_CERTIFICATE_PASSWORD
```

A suggested CI step between packaging and upload:

```yaml
- name: Sign Windows installer
  if: runner.os == 'Windows' && env.WINDOWS_CERTIFICATE != ''
  shell: bash
  env:
    WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
    WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
  run: |
    # Decode WINDOWS_CERTIFICATE to a .pfx, then:
    #   signtool sign /f <certificate>.pfx /p "$WINDOWS_CERTIFICATE_PASSWORD" \
    #     /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 <package>.exe
    # Verify with: signtool verify /pa /v <package>.exe
```

Until signing is configured, the workflow publishes unsigned packages and the
documentation must say so. Do not add self-signed or fake signatures to
production releases.

## Release verification

After publishing, verify the release page contains all four platform packages,
`SHA256SUMS`, and `update.json`, and that `update.json` is reachable at:

```text
https://github.com/Jackiechen259/opencode-quota-checker/releases/latest/download/update.json
```

Then install the previous release and confirm the updater detects the new
version, downloads, verifies, and offers to install it. Check each platform's
install path (NSIS, DMG, AppImage, deb) per the client release smoke test.

The workflow produces unsigned packages until platform signing credentials are
configured. Installed-package smoke tests for launch, tray, notifications,
keyring, and the updater flow are mandatory release gates on every platform.
