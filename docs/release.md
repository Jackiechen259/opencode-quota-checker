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

A pushed `v*` tag runs `.github/workflows/release.yml`, which builds:

- Windows x86-64 NSIS installer
- macOS Intel DMG
- macOS Apple Silicon DMG
- Linux x86-64 deb and AppImage

The publish job combines the packages, creates `SHA256SUMS`, and attaches all
artifacts to the GitHub release. Tags containing a prerelease suffix are marked
as prereleases.

The workflow produces unsigned packages until platform signing credentials are
configured. Installed-package smoke tests for launch, tray, notifications, and
keyring are mandatory release gates on every platform.
