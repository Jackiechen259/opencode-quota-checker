# Iced removal plan (Phase 25)

Executed only after the feature-parity gate (CI green + manual smoke tests).
The frozen Iced implementation stays on `archive/iced-v0.1.2` forever; these
changes only remove it from the live workspace.

## Files to delete

```text
crates/opencode-desktop/            entire crate (src, packager.json, nsis/,
                                    assets/fonts, build.rs, Cargo.toml)
vendor/winit-0.30.13/               entire vendored winit patch
docs/architecture/adr-0001-iced-tray-lifecycle.md
docs/architecture/native-iced.md
```

## Root Cargo.toml

- workspace members: drop `crates/opencode-desktop`
- workspace.dependencies: drop `iced`, `tray-icon`
- `[patch.crates-io]` winit block: delete
- `[workspace.dependencies.windows]` features: keep only what `src-tauri`
  needs (`Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_UI_HiDpi`,
  `Win32_UI_WindowsAndMessaging`); drop `Data_Xml_Dom`, `UI_Notifications`,
  `Win32_System_WinRT` (Iced notification code)
- `raw-window-handle` stays (src-tauri `window/win.rs` uses it)

## xtask

- `release()`: stop updating/committing `crates/opencode-desktop/packager.json`
- `verify_version()`: drop the packager.json comparison
- tests: drop `packager_config_matches_the_release_contract` and the
  packager arm of `workspace_and_packager_versions_stay_in_sync`

## CI / workflows

- `ci.yml`: unchanged (workspace-wide gates; desktop crate simply vanishes)
- `release.yml`: keep `prepare-legacy` for one release cycle (legacy
  `update.json` bridge for installed Iced 0.1.2 clients); it references
  `cargo xtask update-manifest`, which stays

## Docs

- README: drop the migration-status note and the opencode-desktop references
- CONTRIBUTING: drop the Iced-era guidance (already rewritten; verify)
- docs/release.md: drop packager.json mentions
- docs/license-notices.md: drop Iced / tray-icon from the direct-dependency
  list
- docs/tauri-migration/feature-parity.md: mark removal complete

## Post-removal verification

```bash
cargo tree -i iced          # must fail with "package not found"
cargo tree -i winit         # must fail with "package not found"
cargo tree -i tray-icon     # must fail with "package not found"
grep -R "iced" .            # only migration docs / README history
grep -R "tray-icon" .       # only src-tauri Cargo.toml (tauri feature flag)
grep -R "cargo-packager" .  # only migration docs
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
pnpm lint && pnpm typecheck && pnpm test && pnpm build
```
