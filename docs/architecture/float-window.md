# Floating Window Architecture

The floating quota widget is a second Tauri webview window (`label = "float"`)
that stays above other applications. It shares the single `AppState` with the
main window and renders one of three presentations: Full, Compact, or Docked.

This document describes the implementation in `src-tauri/src/float/` and
`src/windows/float/`; it is also the checklist for rebuilding the widget from
scratch.

## 1. Window creation

`float::controller::open` builds the window only when it does not already
exist:

```rust
WebviewWindowBuilder::new(app, "float", WebviewUrl::App("index.html".into()))
    .title("OpenCode Quota Checker · 悬浮窗")
    .inner_size(width, height)      // logical px, from float::layout
    .decorations(false)
    .resizable(false)
    .shadow(false)                  // no tao shadow insets / dark border
    .always_on_top(true)
    .visible(false)                 // sized/positioned before the first frame
```

Key points:

- `.shadow(false)` matters on Windows: with the default undecorated shadow,
  tao enlarges the outer window by the shadow insets (about 6 logical px on
  each side at 100% DPI). The webview only fills the client area, so the insets
  rendered as a thick dark border around the card. With the shadow disabled,
  outer rect and webview surface coincide.
- The window is built hidden, restored to the saved physical position, sized,
  given its window-event handlers and (on Windows) its rounded region, and
  only then shown. The webview's first frame therefore already matches the
  native size.
- On Windows, WebView2 creation **deadlocks** when performed in a synchronous
  command or an event handler. `open_float_window` is therefore async, and the
  tray toggle dispatches to the async runtime instead of building in the menu
  callback. Setup-time restore is safe. `OPEN_LOCK` serializes concurrent
  opens so a double click cannot build the same window label twice.
- The window is not declared in `tauri.conf.json`; it is created on demand.
- `close` goes through the window's close request. `finalize_close` is
  idempotent and is also called directly by the close command, so a missing
  event cannot leave `open = true` behind.
- The float webview receives only `core:event:default` and
  `core:window:allow-start-dragging` in `capabilities/float.json`. Creating,
  sizing, focusing, positioning and closing are Rust-side operations.

## 2. State model

There are exactly two inputs:

| Input | Stored in | Lifetime |
|---|---|---|
| `FloatLayout` (`full` / `compact`) | `AppConfig.float_mode` | Persisted |
| `docked` | `FloatRuntime` | Transient |

`FloatPresentation` (`full` / `compact` / `docked`) is derived once:

```rust
if docked { Docked } else { layout.into() }
```

Everything else is a function of that presentation:

```text
FloatLayout + docked ──> presentation ──> native size
                                      ├─> Windows corner radius (16 / 12 px)
                                      └─> FloatStateDto.presentationMode
```

Invariants:

- `Docked` is never written to `AppConfig`.
- `FloatStateDto.configuredMode` is always the persisted layout;
  `presentationMode` is always the derived presentation; both are built from
  the same `FloatRuntime` guard.
- The native window is never resized from a different copy of the mode; the
  frontend never keeps a local mode variable.
- `float_open` is the persisted *intent* read on launch; `FloatRuntime.open`
  is the live state. `controller::set_open` updates both, and no other code
  writes either.

`FloatStateDto` keeps the original JSON contract:

```json
{ "open": true, "configuredMode": "full", "presentationMode": "docked", "topDocked": true }
```

## 3. Sizes and docking

`float::layout` owns the geometry in logical pixels:

| Presentation | Size |
|---|---|
| Full | 392 × `clamp(134 + 96 × cards, 280, 560)` |
| Compact | 392 × 168 |
| Docked | 392 × 48 |

The Full height is computed from the number of quota windows. Before the first
report the loading skeleton reserves two cards; a report with zero windows
still reserves one.

Docking is a hysteresis decision in physical coordinates:

- window top within 18 px (DPI-scaled) of the monitor work-area top → dock;
- while docked, release only beyond 24 px.

The 18/24 hysteresis prevents the 48 px dock size change from immediately
undocking (or oscillating) at the boundary. On every `Moved` event the
controller:

1. writes the physical position into `AppConfig.float_position`;
2. starts a new position generation for the 750 ms debounce;
3. compares the window top with the work-area top;
4. on a dock-state change, resizes to the presentation, snaps to the work
   area on dock, emits `float://state`;
5. schedules a persistence that only the newest generation may perform.

The position is stored in physical pixels so it survives mixed-DPI layouts;
restore clamps it into the nearest monitor work area.

## 4. Windows native adapter

`float::win` is compiled only on Windows. The webview surface is opaque there,
so CSS `border-radius` alone cannot shape the window; a native window region
is used instead:

- `CreateRoundRectRgn` + `SetWindowRgn` with 16 px for Full/Compact and 12 px
  for Docked (DPI-scaled via `GetDpiForWindow`).
- The region does not follow a resize or DPI change, so the controller
  reapplies it on both `WindowEvent::Resized` and
  `WindowEvent::ScaleFactorChanged`.
- `GetMonitorInfoW` supplies the work area (taskbar-aware) for snapping and
  position restore; `MonitorFromWindow`/`MonitorFromPoint` keep multi-monitor
  coordinates correct.

The CSS radius tokens (`--radius-float` / `--radius-float-docked`) must stay
identical to `float::layout::corner_radius`.

## 5. React rendering

`src/windows/float/FloatWindow.tsx` is a thin shell:

```text
useQuota()          -> report / loading / error
useFloatSnapshot()  -> open / configuredMode / presentationMode / topDocked
                        │
                        ├─ presentationMode === "docked"  -> DockedView
                        ├─ presentationMode === "compact" -> CompactView
                        └─ otherwise                      -> FullView
```

`useFloatSnapshot`:

- fetches `get_float_state` once for the first render;
- listens to `float://state` for every backend mutation;
- keeps a 10 s reconciliation poll as a fallback;
- drops a late initial/poll response when a newer event has arrived;
- applies unchanged snapshots as the same object (no pointless re-render).

Until the first snapshot arrives the window shows a small boot shell, never a
guessed layout.

Dragging uses Tauri's deep drag region rather than bare attributes:

- `.float-header` carries `data-tauri-drag-region="deep"`, so the logo, title,
  status line, spacer and padding all drag the window;
- `.float-docked` does the same for the one-line strip;
- buttons have no drag attribute and are excluded by Tauri's clickable-element
  check, so refresh / mode / close remain clickable.

A bare `data-tauri-drag-region` only drags when the event target is that exact
element; a nested span or logo would silently create a dead zone. That was the
cause of the intermittent "cannot drag" behavior.

## 6. Lifecycle and persistence

```text
open command / tray / startup restore
        └─> controller::open
              ├─ build hidden window
              ├─ restore position, set size, attach handlers, round region
              ├─ show + focus
              └─ set_open(true)  -> FloatRuntime.open + config.float_open -> persist

close command / custom × / Alt+F4
        └─ window close request (and the command's defensive call)
              └─ finalize_close
                    ├─ FloatRuntime.open = false
                    ├─ config.float_open = false
                    ├─ cancel pending position debounce
                    └─ persist + emit float://state
```

`AppState::apply_config` deliberately does not copy `float_open` into the
runtime. `save_config` calls `AppConfig::preserve_float_fields` before writing,
so a Settings save with a stale frontend snapshot cannot overwrite
`float_open` / `float_mode` / `float_position`.

Before exiting, `actions::quit` sets `AppState.shutting_down`. An
exit-triggered float close then only clears the runtime state; it does not
persist `float_open = false`, so the next launch can restore the window.

## 7. Testing

Pure logic is tested on every platform:

- `float/layout.rs`: height clamp/card count, fixed sizes, docking hysteresis
  at 1.0/1.5 DPI, non-positive scale, position clamp, radius mapping.
- `float/state.rs`: presentation derivation, mode transitions, move
  generations, pending-position cancellation.
- `config`: legacy `"docked"` loads as Compact; float fields survive a
  settings round-trip.
- `src-tauri/tests/float_state.rs`: DTO/mode contract against `AppState`.
- `src/windows/float/FloatWindow.test.tsx`: boot shell, all three views,
  event-driven mode switch, stale initial snapshot, controls, empty/error
  states, long labels, extreme percentages.

Manual Windows checks (see `docs/tauri-migration/smoke-test.md`): top-most,
drag, dock hysteresis, mode/position/open restore, mixed-DPI rounded corners,
5+ quota windows, and tray checkmark sync.
