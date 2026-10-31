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
- ☐ Close behavior `MinimizeToTaskbar`: window minimizes to a taskbar button,
  restoring from the taskbar returns the same webview, monitoring continues.
- ☐ Close behavior `Exit`: closing the window exits the whole app.
- ☐ 退出 truly exits: no process or tray remnant.

### Floating window
- ☐ Toggle from the header button and the tray; checkmark stays in sync.
- ☐ Full (392 wide; dynamic height 280–560 by quota count), Compact
  (392×168), Docked top (24 px tall, width fits the content 96–392) and Docked
  side rails (24 px wide, height fits the content 96–560) sizes correct; height
  never clips cards.
- ☐ No dark/black border around the card: the webview surface fills the native
  window exactly (shadow disabled; only the 1px outline + rounded region).
- ☐ Always on top over other apps.
- ☐ Drag by any non-button part of the header (logo, title, status, spacer,
  padding) in Full/Compact, and by any non-button part of a docked strip; every
  button stays clickable.
- ☐ Full → Compact → Full and docked-close; the UI never shows Full inside a
  Compact-sized window (or Full inside a 24 px rail).
- ☐ Docked: mini usage progress bar visible next to the highest-risk window;
  no expand button; × closes. Side rails use vertical writing: every glyph,
  including the percentage digits, stands upright and reads top-to-bottom, and
  neither the text nor the × is clipped.
- ☐ Docked length hugs the content (no wide empty band); a different
  highest-risk label or a longer remaining value resizes the strip without
  clipping the text or the ×.
- ☐ Docked hover card: resting the pointer on a top strip or a side rail grows
  the window into the 392×176 card with three quota rings (5 小时 / 近一周 /
  近一月) and the remaining amount; leaving it collapses back to the same strip
  position. A cursor merely crossing the 24 px strip does not flash the card.
  The card fits on screen when the strip sits near a screen corner, and the
  strip does not drift after collapsing.
- ☐ Restart with `float_open`: the saved mode (Full/Compact) is restored;
  docked presentations are never persisted.
- ☐ Open/close the float window, then save Settings: the tray checkmark and
  `float_open` are not overwritten by the stale settings snapshot.
- ☐ Quit with the float open, relaunch: it reopens. Quit with it closed:
  it stays closed.
- ☐ Position persists across restarts; clamped to a visible monitor.
- ☐ Multi-monitor: moving the float to a second monitor works.
- ☐ Mixed-DPI monitors (100–200%): no WM_DPICHANGED feedback loop, no
  jumps; logical size, region radius and dock thresholds scale.
- ☐ Top docking: dragging near the monitor top (≤18 px, DPI-scaled) snaps to
  the top strip; releasing requires the 24 px hysteresis to leave.
- ☐ Left/right docking: dragging within 18 px of the left/right work-area edge
  (DPI-scaled) snaps to a vertical rail flush with that edge.
- ☐ Side rail drag-out: the rail follows the cursor while the button is held,
  then expands to the configured layout where it was dropped (clamped on
  screen). It must not jump to the opposite edge or stay a thin rail.
- ☐ No immediate re-dock: after a rail is dragged out and released, the expanded
  card does not collapse back into a rail on the first drag pixel; moving it
  clear of the edge and back re-docks it.
- ☐ Dragging along a docked edge repositions the strip and it settles flush to
  that edge on release.
- ☐ Rounded corners (16 px Full/Compact, 12 px Docked capsule) on the float
  card after resize/DPI changes; side rails are capsules on their short axis.
- ☐ Many quota windows (5+): the quota list scrolls inside the window;
  header/meta/footer stay fixed.

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
