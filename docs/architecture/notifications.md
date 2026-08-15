# Notification Architecture

OpenCode Quota Checker delivers threshold alerts through the Tauri
notification plugin (`tauri-plugin-notification`). Threshold evaluation lives
in `opencode-core` (`evaluate_alerts`); the Rust backend
(`src-tauri/src/notifications.rs`) sends each resulting `AlertDecision`
through the system notification service.

Deduplication is handled upstream: `evaluate_alerts` only emits a decision
once per subscription cycle, and `AppState::monitor.last_alerted` persists
that cycle between polls, so the same quota reset period never notifies twice.

## Platform adapters

- **Windows**: WinRT toast notifications (managed by the plugin; the packaged
  app registers its AUMID through the NSIS installer).
- **macOS**: plugin uses `UNUserNotificationCenter`; the app requests
  permission on startup.
- **Linux**: `notify-send` via the plugin; `libnotify4` is declared as the
  Debian package dependency.

## Release gates

Installed-package notification smoke tests remain mandatory release gates:
the adapter must be exercised in a real installed package on Windows, macOS,
and Linux before a release is approved (see
`docs/tauri-migration/smoke-test.md`).
