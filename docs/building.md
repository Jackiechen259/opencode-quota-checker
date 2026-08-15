# Native build guide

## Toolchain

The project requires the latest stable Rust toolchain plus Node.js 22+ and
pnpm 11+ for the React frontend. Ensure `cargo`, `rustfmt`, `clippy`, `node`,
and `pnpm` are available.

## Platform dependencies

### Windows

Install the Visual Studio C++ build tools and the Windows SDK. The
`x86_64-pc-windows-msvc` target is used by CI. WebView2 Runtime is preinstalled
on Windows 10/11.

### macOS

Install Xcode Command Line Tools. CI builds `aarch64-apple-darwin` on a native
Apple Silicon runner.

### Ubuntu 22.04+

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libgtk-3-dev \
  libwebkit2gtk-4.1-dev librsvg2-dev libssl-dev \
  libayatana-appindicator3-dev libxdo-dev libsecret-1-dev libnotify-bin
```

AppImage packaging additionally needs `patchelf` (the Tauri bundler downloads
its own AppImage tooling).

## Commands

```bash
pnpm install
pnpm tauri dev             # dev mode: Vite HMR + Rust rebuilds
pnpm tauri build           # release bundle for the current platform

pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Bundles are written to `src-tauri/target/<target>/release/bundle/`.

### Windows

Build the per-user NSIS installer:

```powershell
pnpm tauri build --bundles nsis
```

The installer requires no administrator rights, shows up under **Settings →
Installed apps**, and creates a Start Menu entry. Installers are unsigned;
SmartScreen may warn.

### Linux

```bash
pnpm tauri build --bundles deb,appimage
```

### macOS (Apple Silicon)

```bash
pnpm tauri build --bundles dmg
```

### Updater signing

Release bundles are signed for the built-in updater. Signing happens inside
`.github/workflows/release.yml` using the `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets. For a local signed build:

```bash
pnpm tauri signer generate -w ~/.tauri/opencode-quota-checker.key
TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/opencode-quota-checker.key \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=<password> \
pnpm tauri build
```

The private key must never be committed; the matching public key is stored in
`src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).

Compilation is not a substitute for installed-package smoke tests. Verify
launch, tray, close-to-tray, notifications, keyring, the floating-window modes
and top docking, and the updater flow on each supported platform — see
[docs/tauri-migration/smoke-test.md](tauri-migration/smoke-test.md).

## Icons

Icon artwork is generated in two steps and needs Pillow (`pip install Pillow`).
The first script draws `assets/icons/icon-source.png` from the parameters at the
top of the file; the second derives every packaged format from it.

```bash
python scripts/generate-icon-source.py
python scripts/generate-icons.py
```

Tauri consumes the PNG/ICO/ICNS files from `src-tauri/icons` (copied from
`assets/icons`); the tray uses `32x32.png` and windows use `icon.png`. After
regenerating, copy the new files into `src-tauri/icons` and rebuild.
