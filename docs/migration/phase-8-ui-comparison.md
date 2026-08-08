# Phase 8 native UI comparison

> **Historical record.** This comparison was written during the Iced native-UI
> migration in the original `volc_status` repository and is retained for
> architectural context.

The Iced implementation preserves the legacy application's user-visible
capabilities without attempting pixel-for-pixel HTML/CSS reproduction.

| Legacy capability | Native Iced implementation |
| --- | --- |
| Credential form | Native text inputs with secure SK rendering and keyring-only persistence |
| Manual refresh | Disabled refresh state while the single request is in flight |
| Three quota cards | Responsive rows/columns with status text, color, metrics, and countdown |
| Settings panel | Native overlay state with validated interval and thresholds |
| Raw response panel | In-memory debug overlay with explicit clipboard action |
| User feedback | Inline errors plus transient copy toast |
| Overlay keyboard behavior | Escape closes settings or debug view |
| Floating window | Shared report, three modes, always-on-top, borderless, draggable |
| High DPI | Iced logical sizes and positions; no physical/logical pixel mixing |
| System tray | Native menu with synchronized floating-window check state |

All actionable controls have visible text labels. Health is communicated by
both text and color, and loading controls cannot start duplicate requests.

