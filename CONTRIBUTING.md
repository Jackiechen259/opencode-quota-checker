# Contributing

## Prerequisites

Use the latest stable Rust toolchain. Install the platform dependencies listed
in `docs/building.md`.

## Development workflow

Create a focused branch and keep commits reviewable. Before opening a pull
request, run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Changes to response parsing, thresholds, configuration, or credential
handling need regression tests. HTTP behavior should use the local mock server
in `crates/opencode-core/tests`; tests must never contain real cookies or
secrets.

Keep platform-specific behavior in `crates/opencode-desktop/src/platform` or
`crates/opencode-desktop/src/window`. State changes must flow through the Iced
message/update path, and blocking I/O must not run on the UI thread.

## Security

Never commit auth cookies, exported keyring data, local configuration files, or
raw production responses. Public error messages must remain bounded and must
not include credentials.

## Releases

Do not edit package versions manually. Follow `docs/release.md` and use
`cargo xtask release`.
