# Iced → Tauri v2 Feature Parity

Status document for the migration of OpenCode Quota Checker from the Iced
desktop client (`crates/opencode-desktop`) to a Tauri v2 application
(`src-tauri` + React frontend). The frozen Iced baseline lives on
`archive/iced-v0.1.2`; all migration work happens on `refactor/tauri-v2`.

Legend: ✅ done · 🚧 in progress · ⬜ not started · ➖ not applicable

## Domain logic (single source of truth: `crates/opencode-core`)

| Feature | Current | Target | Status |
|---|---|---|---|
| OpenCode dashboard HTTP client | `client.rs` | `opencode-core` unchanged | ✅ |
| Dashboard HTML parser (SSR + DOM) | `parser.rs` | `opencode-core` unchanged | ✅ |
| Quota models / window normalization | `models.rs` | `opencode-core` unchanged | ✅ |
| Quota service (fetch + parse) | `quota.rs` | `opencode-core` unchanged | ✅ |
| Threshold validation & alert evaluation | `threshold.rs` | `opencode-core` unchanged | ✅ |
| Keyring credential store | `credential.rs` | `opencode-core` unchanged | ✅ |
| Auth cookie handling | never in UI/logs | never in UI/logs (keyring + request header only) | ✅ |

## Desktop features

| Feature | Current | Target | Status |
|---|---|---|---|
| Dashboard (5h / weekly / monthly quota) | Iced `view/dashboard.rs` | React `pages/Dashboard.tsx` | ✅ |
| Usage overview (highest load, health) | Iced `view/overview.rs` | React `pages/Dashboard.tsx` | ✅ |
| Quota cards, rings, progress bars | Iced components | React components | ✅ |
| Reset countdown (live ticking) | Iced subscription `Tick` | React local timer from `reset_time` | ✅ |
| Refresh / loading / error / empty states | Iced | React | ✅ |
| Credentials screen (login, workspace, cookie) | Iced `view/credentials.rs` | React `pages/Credentials.tsx` | ✅ |
| Credential status (keyring check) | Iced `check_credentials` | Rust command `has_credentials` | ✅ |
| Settings (interval, thresholds, close behavior, float, update toggles) | Iced `view/settings.rs` | React `pages/Settings.tsx` | ✅ |
| Raw JSON debug overlay + copy | Iced `view/debug.rs` | React overlay + clipboard plugin | ✅ |
| Clipboard | Iced `clipboard::write` | `tauri-plugin-clipboard-manager` | ✅ |
| Custom title bar (drag, double-click, min/max/close) | Iced `view/title_bar.rs` | React `components/titlebar` + Tauri window API | ✅ |
| Window maximize state tracking | Iced events | Tauri `onResized`/`onMoved` + `isMaximized` | ✅ |
| Native edge resize of borderless window | winit WNDPROC subclass (`WM_NCHITTEST`) | Tauri `decorations:false` + tao native frame (verify on Windows) | 🚧 |
| Close-to-tray (`MinimizeToTray`) | Iced close request routing | Tauri `onCloseRequested` prevent + hide | ✅ |
| Close behavior `Exit` | Iced | Tauri allow close → app exit | ✅ |
| Tray menu (Show/Hide Main, Toggle Float, Quit) | `tray-icon` + mpsc | Tauri tray + menu events | ✅ |
| Tray left-click shows main | — (menu only) | Tauri `TrayIconEvent::Click` | ✅ |
| Tray float checkmark sync | `set_float_open` | Tauri `MenuItem::set_checked` | ✅ |
| Floating window (Full/Compact/Docked) | Iced `view/float.rs` + `window/float_window.rs` | Tauri `WebviewWindow "float"` + React `FloatWindow.tsx` | ✅ |
| Float always-on-top / borderless / non-resizable | Iced settings | Tauri window config | ✅ |
| Float position persistence + restore | config `float_position` | same config field, Tauri adapter | ✅ |
| Float position clamp to work area | Windows `SetWindowPos` clamp / Iced clamp | Windows adapter + Tauri monitor API | ✅ |
| Top docking (hysteresis 18/24 px, DPI-scaled) | `is_top_docked_at_scale` | ported into Rust float window service | ✅ |
| Snap to monitor top | Windows `SetWindowPos` | Windows adapter | ✅ |
| Rounded float card (Windows) | `CreateRoundRectRgn` | Windows adapter | ✅ |
| Monitoring (interval polling) | Iced subscription `MonitorTick` | Rust tokio background task | ✅ |
| Monitoring while main window hidden | Iced daemon subscriptions | Rust task independent of windows | ✅ |
| Threshold alerts | `evaluate_alerts` | `opencode-core` + Rust task | ✅ |
| Notification dedup per reset cycle | `MonitorState.last_alerted` | Rust `AppState` | ✅ |
| Native notifications | Windows Toast / macOS osascript / Linux notify-send | `tauri-plugin-notification` | ✅ |
| Toast feedback (UI) | Iced toast | React toast | ✅ |
| Update check (GitHub Releases manifest) | custom `update/checker.rs` | `tauri-plugin-updater` (Rust-driven) | ✅ |
| Update download with progress | custom `update/download.rs` | `tauri-plugin-updater` download | ✅ |
| Auto-download when enabled | `auto_download_updates` | Rust updater service | ✅ |
| Install confirmation | Iced banner + settings button | React confirm dialog → Rust install | ✅ |
| Installer launch (NSIS / DMG / AppImage / deb) | custom `update/installer.rs` | `tauri-plugin-updater` install | ✅ |
| Release notes in browser | custom `open_url` | Rust launcher (`open_url`) | ✅ |
| App icon (window/tray/exe) | assets/icons + build.rs winresource | Tauri icons (same artwork) | ✅ |
| Chinese UI copy | Iced strings | React strings (same copy) | ✅ |

## Config & credentials compatibility

| Item | Requirement | Status |
|---|---|---|
| Config path | `%APPDATA%/opencode-quota-checker/config.json` (same `directories` discovery) | ✅ |
| Config fields | workspace id, monitor enabled/interval, thresholds, close behavior, float open/mode/position, update toggles | ✅ |
| Schema 1 & 2 configs load | `#[serde(default)]` migration (no silent loss) + tests | ✅ |
| Keyring credential reuse | `service=opencode-quota-checker`, `account=opencode-auth` (unchanged `opencode-core`) | ✅ |
| No secrets in WebView | auth cookie only in Rust keyring; cleared from React state after save | ✅ |

## Build & release

| Item | Current | Target | Status |
|---|---|---|---|
| Workspace | `crates/opencode-core`, `crates/opencode-desktop`, `xtask` | + `src-tauri`, eventually − desktop | ✅ (desktop kept as fallback) |
| Frontend build | — | Vite + React + TS, `pnpm build` | ✅ |
| Windows packaging | cargo-packager NSIS (currentUser, EN + zh-CN) | Tauri NSIS (same modes/languages) | 🚧 (config present, not built yet) |
| Linux packaging | deb + AppImage | Tauri deb + AppImage | 🚧 |
| macOS packaging | dmg | Tauri dmg (aarch64 only) | 🚧 |
| Release assets | `update.json` + SHA256SUMS + installers | + `latest.json` + `.sig` (Tauri updater), keep legacy `update.json` for one cycle | 🚧 |
| Signing | — (SHA-256 only) | Tauri updater signing key (private key only in GitHub Secrets) | 🚧 |
| CI | Rust fmt/clippy/test + native build matrix | + pnpm install/lint/typecheck/build, Tauri builds | ⬜ |
| Version sync | workspace + packager.json | workspace + src-tauri + package.json + tauri.conf.json via `xtask` | ⬜ |
| winit vendor patch | `vendor/winit-0.30.13` (Iced-only) | removed after Iced removal (`cargo tree` check) | ⬜ |

## Upgrade path (Iced 0.1.2 → first Tauri release)

1. Old Iced client polls `update.json` (schema 1) and downloads the NSIS
   installer of the first Tauri release. Tauri NSIS installs over the old
   app (same identifier), preserving `%APPDATA%\opencode-quota-checker\config.json`
   and the keyring credential.
2. First Tauri release also publishes `latest.json` + signatures so future
   Tauri releases update through `tauri-plugin-updater`.
3. Legacy `update.json` generation is kept in `xtask update-manifest` and the
   release workflow for at least one release cycle after the Tauri switch.
