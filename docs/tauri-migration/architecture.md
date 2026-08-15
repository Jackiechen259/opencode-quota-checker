# Tauri Architecture

OpenCode Quota Checker after the Iced → Tauri v2 migration.

```text
React / TypeScript UI (src/)
        │  Tauri IPC commands + events
        ▼
Tauri Rust Desktop Layer (src-tauri/src/)
        ├── commands/       typed IPC surface (quota, config, credentials,
        │                   monitor, float, update, app status)
        ├── monitor.rs      tokio background monitor task
        ├── tray.rs         system tray + menu events
        ├── window/         main + floating windows, top docking,
        │                   Windows native adapters (work area, snap, corners)
        ├── updater.rs      tauri-plugin-updater state machine
        ├── notifications.rs native notifications (dedup upstream)
        ├── state.rs        AppState — the single shared state container
        └── config/         config store (Iced-compatible path + schema)
        ▼
opencode-core (crates/opencode-core/)
        ├── client.rs       OpenCode Go HTTP (auth cookie as request header)
        ├── parser.rs       SSR + DOM dashboard parsing
        ├── quota.rs        fetch + parse service
        ├── threshold.rs    alert evaluation (per-cycle dedup)
        ├── credential.rs   keyring store (service=opencode-quota-checker,
        │                   account=opencode-auth)
        └── models.rs       UsageReport / WindowReport
```

## Principles

1. **Single source of truth**: every piece of business logic lives in
   `opencode-core` or the Rust backend. React never talks to OpenCode,
   parses HTML, touches the keyring, or reimplements quota math.
2. **Rust owns the state**: `AppState` (config, usage, monitor, float,
   updater, credentials, tray) is shared by every window and background task.
   The frontend receives snapshots (`get_*` commands) and event streams and
   sends commands; it never mirrors state it owns.
3. **Secrets stay in Rust**: the auth cookie is read from the keyring inside
   the backend, transmitted only as a request header, and never returned to
   the webview, logged, or written to config.
4. **The monitor is a background task**, not a frontend timer: it keeps
   polling while the main window is hidden to the tray.

## Data flow

- Quota: `monitor::run_once` (or the `refresh_usage` command) → keyring →
  `QuotaService::fetch_quota` → `evaluate_alerts` (dedup) → notifications →
  `quota://updated` / `quota://error` / `monitor://status` events.
- Config: React form → `save_config` command → `AppConfig::validate` (final
  authority) → atomic file write → `app://status` event.
- Float window: commands create/destroy/resize it; Rust `on_window_event`
  handles position persistence (debounced), top docking (hysteresis 18/24 px,
  DPI-scaled, physical coordinates), snapping, and Windows rounded corners.

## Events

| Event | Payload | Emitted when |
|---|---|---|
| `quota://updated` | `UsageReport` | every successful fetch |
| `quota://error` | `AppError` | every failed fetch |
| `monitor://status` | `MonitorStatusDto` | monitor state changes |
| `float://state` | `FloatStateDto` | float window changes |
| `update://state` | `UpdateStateDto` | updater state changes |
| `app://status` | `AppStatusDto` | credential/config mutations |

## Security / capabilities

- `main` window: `core:default` + window controls + clipboard write.
- `float` window: least privilege — event listen, drag, resize/move/close
  itself only. No credential, updater, fs, or shell permissions.
- No secrets in the webview; no shell execution from the frontend.

## Upgrade compatibility

- Config: same path (`opencode-quota-checker/config.json` via
  `directories`), same schema 1/2 semantics with `#[serde(default)]`
  migration; verified by tests.
- Credentials: same keyring service/account names; an Iced-saved cookie is
  readable by the Tauri build (verified end-to-end).
- Updater: legacy `update.json` bridge published alongside Tauri
  `latest.json` for the first release cycle.
