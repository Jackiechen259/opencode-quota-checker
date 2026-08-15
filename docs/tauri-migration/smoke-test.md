# Tauri Migration — Manual Smoke Tests

Checklist for validating the Tauri v2 build before every release and before
the Iced client is removed. Run through the **Windows** section on a Windows
11 machine with at least one secondary monitor at a different DPI if possible.
Linux/macOS sections apply once those bundles exist.

Legend: ☐ unchecked · ☑ passed · ❌ failed (record details)

## Windows

### Startup & shell
- ☐ Startup without a console window (release build).
- ☐ Taskbar icon shows the project icon.
- ☐ Main window opens centered, borderless, with the custom title bar.
- ☐ App icon + "OpenCode Quota Checker" title in the title bar.
- ☐ Drag the title bar moves the window; double-click toggles maximize.
- ☐ Minimize / maximize / restore buttons work.
- ☐ Windows Snap (Win+Arrow / drag to edge) works; glyph updates.
- ☐ Resize from every edge and corner of the borderless window.
- ☐ High-DPI scaling looks sharp (150 %/200 %).

### Config & credentials upgrade
- ☐ With an existing Iced `%APPDATA%\opencode-quota-checker\config.json`
  (schema 1 or 2): all fields survive (workspace id, monitor, thresholds,
  close behavior, float mode/position, update toggles).
- ☐ With an existing Iced keyring credential (`opencode-auth` under
  `opencode-quota-checker`): the dashboard shows quota without re-entering
  the cookie. This is the critical upgrade acceptance item.
- ☐ Fresh install: default config written on first run; credentials page
  shown until configured.

### Dashboard
- ☐ 5-hour / weekly / monthly cards render with ring, metrics, reset
  countdown; countdown ticks every second.
- ☐ Overview hero shows highest load + health distribution.
- ☐ Manual refresh updates data; loading spinner shows while in flight.
- ☐ Error state (network down / expired cookie) shows the warning notice
  without losing the previous report.
- ☐ Raw JSON overlay: loads, copies to clipboard, closes with Esc.

### Monitoring & notifications
- ☐ Periodic polling runs at the configured interval.
- ☐ Polling continues while the main window is hidden to the tray.
- ☐ Threshold alerts fire once per reset cycle (dedup): no repeated
  notifications while the quota stays above the threshold.
- ☐ Notifications appear in the Windows notification center.

### Tray & close-to-tray
- ☐ Tray icon visible with tooltip; right-click opens the menu.
- ☐ Left-click shows/focuses the main window.
- ☐ Menu: 打开主窗口 / 隐藏主窗口 / 显示悬浮窗 (checkmark) / 退出.
- ☐ Close (×) with `MinimizeToTray`: window hides, process keeps running,
  tray restore works, monitoring continues.
- ☐ Close behavior `Exit`: closing the window exits the whole app.
- ☐ 退出 truly exits: no process or tray remnant.

### Floating window
- ☐ Toggle from the header button and the tray; checkmark stays in sync.
- ☐ Full (360×420), Compact (360×148), Docked (360×56) sizes correct.
- ☐ Always on top over other apps.
- ☐ Drag by the brand area (Full/Compact) or the whole strip (Docked).
- ☐ Docked: "展开" returns to Full; × closes.
- ☐ Position persists across restarts; clamped to a visible monitor.
- ☐ Multi-monitor: moving the float to a second monitor works.
- ☐ Mixed-DPI monitors: no WM_DPICHANGED feedback loop, no jumps.
- ☐ Top docking: dragging near the monitor top (≤18 px, DPI-scaled) snaps to
  Docked; releasing requires the 24 px hysteresis to leave.
- ☐ Rounded corners on the float card after resize/DPI changes.

### Updater
- ☐ 检查更新 finds a newer release from GitHub.
- ☐ Auto-download toggle downloads in the background with progress.
- ☐ Install requires explicit user confirmation (banner button / settings).
- ☐ NSIS silent install replaces the app; signature verification passed
  before install.
- ☐ Iced → Tauri upgrade path: legacy `update.json` points at the new NSIS
  installer and the installer preserves config + keyring credential.

## Linux (x86_64)

- ☐ AppImage and deb launch; tray + notifications work on X11 and Wayland.
- ☐ Floating window, docking, and position persistence work.
- ☐ Updater: AppImage in-place replacement works; deb is not auto-installed
  (plugin limitation; documented).

## macOS (Apple Silicon)

- ☐ dmg launches; tray + notifications work.
- ☐ Floating window, docking, and position persistence work.
- ☐ Updater: DMG opens after confirmation; signed package verified.

## Regression

- ☐ `cargo xtask verify-version` passes before release.
- ☐ CI green: fmt, clippy -D warnings, rust tests, pnpm lint/typecheck/test.
- ☐ Windows / Linux / macOS ARM production bundles build.
- ☐ Iced client still builds (until removed) and remains on
  `archive/iced-v0.1.2`.
