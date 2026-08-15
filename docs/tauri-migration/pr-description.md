# OpenCode Quota Checker — Iced → Tauri v2

Refactors the desktop client from Iced to **Tauri v2 + React + TypeScript + Vite**,
keeping `opencode-core` as the single source of truth for all business logic.

## What changed

- **New architecture**: React/TS frontend (`src/`) ↔ typed IPC (`src-tauri/src/commands/`)
  ↔ shared Rust `AppState` ↔ unchanged `opencode-core`.
- **Rust backend owns everything**: config store, keyring credentials,
  tokio background monitor (keeps polling while hidden to tray), threshold
  alerts with per-cycle dedup, system tray, close-to-tray, floating window
  (Full/Compact/Docked) with top docking + position persistence, native
  notifications, `tauri-plugin-updater` state machine.
- **Upgrade compatible**: same `config.json` path/schema (1 & 2 load with
  migration tests), same keyring namespace
  (`opencode-quota-checker` / `opencode-auth`) — verified live on Windows
  with real user data (quota fetch succeeded, `windows=3`).
- **Signing**: Tauri updater Ed25519 keypair; public key in
  `tauri.conf.json`, private key only in GitHub Actions Secrets.
- **Iced removed**: `crates/opencode-desktop`, `vendor/winit-0.30.13` patch,
  cargo-packager config deleted. Frozen forever on
  `archive/iced-v0.1.2` (regression + emergency rollback).
- **CI/Release**: pnpm + cargo gates on 3 platforms; `tauri-action` bundles
  (NSIS/deb/AppImage/dmg) with updater signatures + merged `latest.json`;
  legacy `update.json` bridge kept for one release cycle so installed Iced
  0.1.2 clients can upgrade. `xtask` keeps workspace/package.json/
  tauri.conf.json versions in sync.

## Feature Parity Checklist

- [x] OpenCode quota fetch works (live-verified)
- [x] 5-hour / weekly / monthly quota windows
- [x] used / remaining / reset countdown
- [x] credentials survive Iced → Tauri upgrade (keyring, live-verified)
- [x] config survives Iced → Tauri upgrade (live-verified)
- [x] manual refresh / periodic monitoring / monitoring while window hidden
- [x] threshold alerts + notification dedup per reset cycle
- [x] system tray: show/hide main, toggle float, quit
- [x] close-to-tray + tray restore
- [x] custom titlebar: drag, minimize, maximize/restore (Snap/Win+Arrow
  track via native events)
- [x] floating window Full / Compact / Docked
- [x] top docking (hysteresis 18/24 px, DPI-scaled, physical coords)
- [x] float position persistence + clamp to work area
- [x] raw JSON debug + clipboard
- [x] signed updater: check → download → user confirmation → install
- [x] CI green (fmt/clippy/tests ×3 platforms, pnpm gates)
- [x] Windows / Linux / macOS ARM release builds

## Test results

- `cargo test --workspace`: 56 tests green (opencode-core 19+8 HTTP,
  src-tauri lib 13, integration 7+1, xtask 8)
- Frontend: 18 vitest tests, `pnpm lint/typecheck/build` clean
- Windows live smoke: boot, config+keyring upgrade, real quota fetch
- Notable bugs found by the new tests and fixed: reentrant-Mutex
  deadlocks in `push_monitor_config`/`status_dto`; Windows test binaries
  missing the Common-Controls v6 manifest
  (`src-tauri/build.rs`)

## Migration docs

`docs/tauri-migration/` — architecture, feature-parity, config-migration,
updater-migration, smoke-test, iced-removal.
