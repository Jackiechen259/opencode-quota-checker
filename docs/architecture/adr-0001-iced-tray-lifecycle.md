# ADR 0001: Iced daemon and tray lifecycle

- Status: accepted for implementation, pending cross-platform runtime smoke tests
- Date: 2026-07-24

## Context

OpenCode Quota Checker must remain alive after all Iced windows are closed and must be
recoverable from a native system tray. `tray-icon` has platform event-loop and
main-thread requirements, while Iced owns its `winit` event loop.

## Decision

Use `iced::daemon` as the application runtime. Create the tray service during
daemon boot, keep the `TrayIcon` and menu items alive in application state, and
forward `MenuEvent` values into an `std::sync::mpsc` channel. A lightweight Iced
time subscription wakes the daemon and drains that channel.

Main-window "hide" is implemented as close-and-recreate because Iced 0.14 does
not expose a cross-platform window visibility command. The daemon keeps running
without windows, and `window::open` recreates the main window from the tray.

If tray initialization fails, close behavior changes to process exit. This
prevents an unrecoverable background process.

## Reasons

- Iced 0.14 explicitly keeps a daemon alive with zero windows and exits only
  after an `iced::exit` task.
- `window::open` is a public, supported multi-window API.
- `MenuEvent::set_event_handler` is the tray crate's documented integration
  point for event-loop applications.
- The polling subscription does not access private Iced/winit APIs and remains
  active with zero windows.
- All tray actions still enter the application's `Message` update path.

## Consequences

- The application uses close-and-recreate rather than an invisible main window.
- Tray menu latency is bounded by the 100 ms subscription period.
- Platform runtime tests remain mandatory. A successful compile does not prove
  that a particular Linux desktop exposes an AppIndicator-compatible tray.
- If macOS or Linux runtime testing shows that tray creation must happen later
  than daemon boot, the thin custom-winit fallback described in the refactor
  plan will be used without changing core or view state.

## Spike

The compilable prototype is in `spikes/iced-tray-daemon`. Run it with:

```bash
cargo run --manifest-path spikes/iced-tray-daemon/Cargo.toml
```

Manual acceptance:

1. Close the main window and verify the process remains.
2. Use the tray menu to recreate the main window.
3. Hide it again from the tray menu.
4. Exit from the tray and verify the process terminates.
5. Repeat on Windows, macOS, and Ubuntu.

