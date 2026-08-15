# Contributing

## Prerequisites

Use the latest stable Rust toolchain plus Node.js 22+ and pnpm 11+. Install the
platform dependencies listed in `docs/building.md`.

## Development workflow

Create a focused branch and keep commits reviewable. Before opening a pull
request, run:

```bash
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

CI runs the same gates on Windows, Linux, and macOS ARM.

Changes to response parsing, thresholds, configuration, or credential
handling need regression tests. HTTP behavior should use the local mock server
in `crates/opencode-core/tests`; tests must never contain real cookies or
secrets.

Keep business logic in `crates/opencode-core`; keep window/tray/monitor
behavior in `src-tauri/src`; keep UI in `src/`. State changes must flow
through the Rust `AppState` and IPC commands — the React frontend must never
implement quota/credential logic.

## Windows test binaries

`src-tauri` links `muda`/tauri dialog code that imports comctl32 v6 entry
points. The packaged app resolves them through tauri-build's application
manifest, but test harness binaries need the Common-Controls v6 dependency
embedded too — `src-tauri/build.rs` does this with
`/MANIFEST:EMBED` + `/MANIFESTINPUT`. If the linker starts failing with
`STATUS_ENTRYPOINT_NOT_FOUND` or duplicate-resource errors, check that
`build.rs` and the `tests/` targets are intact.

## Security

Never commit auth cookies, exported keyring data, local configuration files,
raw production responses, or the Tauri updater signing private key. Public
error messages must remain bounded and must not include credentials. The
floating window capability must stay least-privilege (see
`src-tauri/capabilities/float.json`).

## Releases

Do not edit package versions manually. Follow `docs/release.md` and use
`cargo xtask release`. Updater signing keys live only in GitHub Actions
Secrets (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`).
