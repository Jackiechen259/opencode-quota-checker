# Native build guide

## Toolchain

The project supports the latest stable Rust toolchain. Install it with rustup
and ensure `cargo`, `rustfmt`, and `clippy` are available.

## Platform dependencies

### Windows

Install the Visual Studio C++ build tools and the Windows SDK. The
`x86_64-pc-windows-msvc` target is used by CI.

### macOS

Install Xcode Command Line Tools. CI builds `aarch64-apple-darwin` on a native
Apple Silicon runner.

### Ubuntu 22.04+

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libgtk-3-dev \
  libayatana-appindicator3-dev libxdo-dev libsecret-1-dev libnotify-bin
```

AppImage packaging additionally needs `squashfs-tools`.

## Commands

```bash
cargo run -p opencode-desktop
cargo test --workspace
cargo build --workspace --release
```

To produce the current platform's default install format:

```bash
cargo install cargo-packager --locked
cargo packager --release --config crates/opencode-desktop/packager.json
```

Packages are written to `target/packages`.

Compilation is not a substitute for installed-package smoke tests. Verify
launch, window recreation from the tray, process exit, notifications, keyring,
and all floating-window modes on each supported platform.

## Icons

Icon artwork is generated in two steps and needs Pillow (`pip install Pillow`).
The first script draws `assets/icons/icon-source.png` from the parameters at the
top of the file; the second derives every packaged format from it.

```bash
python scripts/generate-icon-source.py
python scripts/generate-icons.py
```

`window.rgba` and `tray.rgba` are raw RGBA embedded with `include_bytes!`, so
rebuild the desktop crate after regenerating. Re-check the 16px `icon.ico` frame
and `tray.rgba` whenever the artwork changes — the tray renders at those sizes
and small-scale legibility is what constrains the design.
