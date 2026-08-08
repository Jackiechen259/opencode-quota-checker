# Phase 10 notification dependency note

> **Historical record.** This decision note was written during the Iced
> native-UI migration in the original `volc_status` repository. The platform
> notification adapters it describes are still used by OpenCode Quota Checker.

The initial native implementation used `notify-rust`, as proposed by the
refactor plan. On Windows, that crate resolves a standalone WinRT helper whose
package name contains the removed framework's name. Although it did not bring
in a webview or the framework runtime, it made the Phase 10 legacy dependency
gate ambiguous and caused the literal repository grep to fail.

Phase 10 therefore uses thin platform adapters:

- Windows: WinRT toast APIs through the Microsoft `windows` crate
- macOS: the built-in `osascript` notification command
- Linux: `notify-send`, declared as the `libnotify-bin` package dependency

Installed-package notification smoke tests remain mandatory release gates.
