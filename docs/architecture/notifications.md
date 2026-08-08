# Notification Architecture

OpenCode Quota Checker uses platform-specific native notification adapters to
deliver threshold alerts. Threshold evaluation lives in `opencode-core`
(`evaluate_alerts`), and the desktop app sends each resulting
`AlertDecision` through `crates/opencode-desktop/src/platform/notification.rs`.

Thin shell adapters are used instead of a cross-platform notification crate:
they add no extra native runtime dependencies and keep the notification path
inspectable and debuggable on each platform.

## Platform adapters

- **Windows**: WinRT toast APIs through the Microsoft `windows` crate. The
  toast body is an XML `ToastGeneric` template, text is XML-escaped, and
  delivery runs under a multithreaded COM apartment.
- **macOS**: the built-in `osascript` notification command
  (`display notification ... with title ...`).
- **Linux**: `notify-send` with the app name `OpenCode Quota Checker`;
  `libnotify-bin` is declared as the package dependency for Debian packages.

## Release gates

Installed-package notification smoke tests remain mandatory release gates:
the adapter must be exercised in a real installed package on Windows, macOS,
and Linux before a release is approved.
