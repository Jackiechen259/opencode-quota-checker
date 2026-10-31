# Floating Window Architecture

The floating quota widget is a second Tauri webview window (`label = "float"`)
that stays above other applications. It shares the single `AppState` with the
main window and renders one of five presentations: Full, Compact, or a docked
strip on the top, left or right work-area edge. A docked strip expands into a
three-ring hover card while the pointer rests on it.

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

There are exactly three presentation inputs:

| Input | Stored in | Lifetime |
|---|---|---|
| `FloatLayout` (`full` / `compact`) | `AppConfig.float_mode` | Persisted |
| `dock_edge` (`top` / `left` / `right`) | `FloatRuntime` | Transient |
| Measured strip length | `FloatRuntime.docked_length` | Transient |

The dock edge is a single value, so "docked twice" is unrepresentable. The
runtime also carries two drag-time flags:

| Flag | Meaning |
|---|---|
| `undock_pending` | A side rail has left its release band but keeps the rail presentation until the drag settles. |
| `dock_guarded` | Docking is disarmed after an un-dock until the window leaves every snap band. |

The webview measures its rendered content (text widths depend on the platform
fonts) and reports the strip length along its axis through
`set_float_docked_length`. The controller clamps it and uses it only while
docked; it never changes the presentation and never appears in the DTO or the
config. The same measurement feeds the top strip's **width** and a side
rail's **height**.

`FloatPresentation` is derived once:

```rust
match dock_edge {
    Some(edge) => edge.into(),      // docked_top | docked_left | docked_right
    None => layout.into(),          // full | compact
}
```

The presentation drives the layout, the Windows radius and the DTO:

```text
FloatLayout + dock edge ──> presentation ──> native size
                                          ├─> Windows corner radius (16 / 12 px)
                                          └─> FloatStateDto.presentationMode

Measured strip length ──> native length (docked only; no DTO, no config)
```

Invariants:

- Docked presentations are never written to `AppConfig`.
- `FloatStateDto.configuredMode` is always the persisted layout;
  `presentationMode` is always the derived presentation; both are built from
  the same `FloatRuntime` guard.
- The native window is never resized from a different copy of the mode; the
  frontend never keeps a local mode variable.
- `float_open` is the persisted *intent* read on launch; `FloatRuntime.open`
  is the live state. `controller::set_open` updates both, and no other code
  writes either.
- The strip length is transient and presentation-neutral: it only feeds the
  native size while an edge is docked.
- `pending` is only meaningful while docked to a side edge; the docking
  decision is applied as one `DockPlan` so edge, pending and guard cannot
  disagree.

`FloatStateDto` keeps the original JSON contract shape minus the redundant
edge flag — the presentation name already carries the edge:

```json
{ "open": true, "configuredMode": "full", "presentationMode": "docked_right", "dockHover": false }
```

### Docked hover card

`FloatRuntime.dock_hover` is a transient presentation *flag*, not a sixth
presentation name:

- it is only representable while `dock_edge` is `Some`. `apply_dock_plan`
  clears it whenever the window leaves the docked presentation — or starts to
  leave it (`pending`) — and closing/reopening the window clears it too;
- it never enters `AppConfig` and never changes `configuredMode` or
  `presentationMode`;
- it crosses the IPC boundary as `dockHover`, because the webview can only
  render the card the native window has already grown into. The frontend only
  *requests* the flag (§5); it never resizes itself.

The runtime also carries `dock_hover_origin`, the coordinate along the docked
edge the strip had before expanding. Expanding may clamp the card back into
the work area, so collapsing restores this anchor instead of the card's
position; the strip cannot drift just because the card had to fit on screen.
On Windows, dragging the expanded card along its edge updates the anchor (the
controller probes the mouse button, like the deferred side un-dock), so
collapsing lands where the card was dropped.

## 3. Sizes and docking

`float::layout` owns the geometry in logical pixels:

| Presentation | Size |
|---|---|
| Full | 392 × `clamp(134 + 96 × cards, 280, 560)` |
| Compact | 392 × 168 |
| Docked (top) | `clamp(measured + 46, 96, 392)` × 24 |
| Docked (left/right) | 24 × `clamp(measured + 46, 96, 560)` |
| Docked hover card (any edge) | 392 × 176 |

The hover card is deliberately modest: it keeps the 392 px width of the other
layouts so the overview never covers a whole screen corner, and its 176 px
height fits the header row plus three 84 px rings with their labels. It is
smaller than the Full layout (which starts at 280 px) and much larger than the
24 px strip it replaces.

Expanding and collapsing are anchored transitions, resolved by
`float::layout::docked_position`:

- the coordinate along the docked edge (x for the top strip, y for a rail) is
  preserved and clamped into the work area;
- the owning axis stays flush to the edge: a top card grows downward from
  `work.top`, a left card rightward from `work.left`, a right card leftward
  from `work.right - 392`.

`controller::set_dock_hover` applies the new size and position together and
records the state before resizing, so the `Resized` handler already rounds the
window with the target radius. Hover requests are ignored when the window is
not docked, while a side rail waits for its un-dock to settle (the native move
loop owns the window), and while the primary mouse button is down.

The Full height is computed from the number of quota windows. Before the first
report the loading skeleton reserves two cards; a report with zero windows
still reserves one.

Every docked strip is content-fitted so it does not leave a wide empty band on
screen. The webview measures an out-of-flow copy of the strip content at its
natural width, adds the 46 px chrome (8 px padding × 2, 1 px outline × 2, two
4 px gaps and the 20 px close button) and reports the total through
`set_float_docked_length`; `float::controller` clamps it and applies it to the
strip axis. A side rail's default length before the first report is 280 px.

Docking is deliberately the smallest presentation: 24 px on the short axis (1 px
outline plus the 20 px close button), so the always-on-top strip covers as
little as possible of the windows behind it. At 24 px the 12 px corner radius
renders as a capsule.

Docking is a hysteresis decision in physical coordinates:

- window edge within 18 px (DPI-scaled) of the work-area edge → dock;
- while docked, release only beyond 24 px.

Edge distances are `window.top - work.top`, `window.left - work.left` and
`work.right - window.right`; a negative distance means the window was dragged
past that edge. `float::layout::plan_dock` resolves one move:

1. a docked window stays while its edge is inside the release band;
2. a docked window that leaves the band and is a **top** strip un-docks
   immediately — the configured layout grows downward from the top-left corner,
   so it stays on screen;
3. a **side** rail that leaves the band stays a rail with `pending = true`. The
   native move loop owns the window position while the mouse button is down
   (Windows re-applies its own tracked rectangle on every mouse move), so
   expanding a right-edge rail mid-drag would put most of the card off-screen;
4. an un-docked, unguarded window docks to the **top** first (preserving the
   long-standing top-strip behavior in the corners), otherwise to the nearest
   side;
5. an un-docked **guarded** window cannot dock until it is clear of every snap
   band. Without this, a window un-docked while flush against an edge would
   collapse straight back into a rail on the next move event.

On every `Moved` event the controller:

1. writes the physical position into `AppConfig.float_position`;
2. starts a new position generation for the 750 ms debounce and the settle pass;
3. applies the resolved `DockPlan`; if the presentation changed, resizes to it,
   snaps docked windows flush to their edge and emits `float://state`;
4. schedules the 220 ms dock settle for docked windows;
5. schedules a persistence that only the newest generation may perform.

The settle pass waits for the newest move generation and — on Windows — for the
primary mouse button to be released (`GetAsyncKeyState`), so it never fights the
native move loop. It then:

- re-snaps a docked window flush to its edge (the move loop can leave a small
  offset behind), or
- commits a pending side un-dock: if the rail was dropped against another edge
  it docks there instead; otherwise it expands to the configured layout, is
  clamped into the work area, and arms the guard while the result still touches
  an edge.

The position is stored in physical pixels so it survives mixed-DPI layouts;
restore clamps it into the nearest monitor work area.

## 4. Windows native adapter

`float::win` is compiled only on Windows:

- `GetMonitorInfoW` supplies the work area (taskbar-aware) for the docking
  thresholds and the position restore; `MonitorFromWindow`/`MonitorFromPoint`
  keep multi-monitor coordinates correct.
- `restore_position` chooses the monitor from the saved point so a stale
  position cannot strand the window on a monitor that no longer exists.
- `primary_button_down` (`GetAsyncKeyState`) tells the settle pass whether the
  native move loop is still running.
- The webview surface is opaque there, so CSS `border-radius` alone cannot shape
  the window; a native window region is used instead:
  `CreateRoundRectRgn` + `SetWindowRgn` with 16 px for Full/Compact and the
  docked hover card, 12 px for every collapsed docked strip (DPI-scaled via
  `GetDpiForWindow`).
- The region does not follow a resize or DPI change, so the controller
  reapplies it on both `WindowEvent::Resized` and
  `WindowEvent::ScaleFactorChanged`.

Other platforms use the monitor rectangle as the work area and Tauri's
`set_position` for the same snapping; the settle pass relies on the 220 ms
delay alone because there is no portable mouse-button probe.

The CSS radius tokens (`--radius-float` / `--radius-float-docked`) must stay
identical to `float::layout::corner_radius`.

## 5. React rendering

`src/windows/float/FloatWindow.tsx` is a thin shell:

```text
useQuota()          -> report / loading / error
useFloatSnapshot()  -> open / configuredMode / presentationMode / dockHover
                        │
                        ├─ presentationMode starts with "docked_" -> DockedWindow(edge, dockHover)
                        │                       ├─ dockHover             -> DockedHoverCard
                        │                       └─ otherwise            -> DockedView
                        ├─ presentationMode === "compact"          -> CompactView
                        └─ otherwise                              -> FullView
```

`dockedEdge()` in `floatLayout.ts` maps the presentation name to the edge; the
edge is never tracked separately.

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
- `.float-docked` does the same for every docked strip;
- buttons have no drag attribute and are excluded by Tauri's clickable-element
  check, so refresh / mode / close remain clickable (docked strips only contain
  ×).

Docked is a read-only status strip: a health dot, the highest-risk window label,
a 56 × 4 px usage progress bar, the percent and the remaining quota. It
deliberately has no expand button. Two gestures change its presentation:
dragging it away from its edge un-docks it and restores the configured
Full/Compact layout, and resting the pointer on it expands it into the hover
card (below). `DockedView` also measures an out-of-flow `max-content` clone of
that content after every report change and calls `set_float_docked_length`, so
the native size hugs the label, percent and remaining quota instead of a fixed
392 px.

### Hover card

`DockedWindow` owns the hover *intent*, never a layout:

- `pointerenter` starts a 140 ms expand delay and `pointerleave` a 260 ms
  collapse delay; either event cancels the other timer, so a cursor crossing
  the 24 px strip cannot flash the card and a boundary flicker cannot close it.
  A leave with a mouse button still held is ignored: the strip is a drag
  region and the docking decision owns that transition.
- the timers only call `set_float_dock_hover`. The card renders when the
  `float://state` snapshot reports `dockHover`, exactly like
  `presentationMode` — the webview never resizes anything and keeps no hover
  state. Both requests are idempotent, and the backend ignores them while the
  window is undocked, while a side un-dock is pending, or while the user is
  dragging.
- `DockedHoverCard` draws one SVG ring per quota window. `dockHoverWindows`
  orders the 5-hour / weekly / monthly windows canonically, fills spare rings
  with any extra window and caps the card at three; a missing window is
  omitted rather than drawn as an empty ring. The arc is a single
  full-circumference dash offset by `ringDashOffset`, which reuses
  `progressWidth` — the ring shows the same "used" percentage as every other
  view, in that window's own health color, with the remaining quota and the
  label underneath.
- the card header (`.float-dock-card-header`) is the drag region, so the
  expanded window stays draggable; the close button is excluded like every
  other Tauri clickable element.

All three dock edges share the same markup. The side rails switch the strip to
vertical writing (`.float-docked--left` / `--right` set
`writing-mode: vertical-rl` and `text-orientation: upright`;
`.float-docked-content--side` carries the same mode for the measurement clone):
the flex row runs top-to-bottom and every glyph — Chinese characters and the
percentage digits alike — stands upright, so the readouts run down the rail
instead of lying on their side. The native window stays simply 24 px wide × the
measured length tall — there is no transform and no pre-transform overflow to
clip. Because upright digits need a full em each, a side rail is taller than the
equivalent top strip (the measured content grows accordingly). The 8 px padding
moves from the sides to the strip ends (it is length chrome, not thickness), and
the progress meter is transposed: 4 px wide × up to 56 px tall with its fill
driven by React on the height axis. The measuring clone is appended to
`document.body` so it is measured outside the rail's fixed native box, and it
carries the side class so the vertical text metrics match.

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
                    ├─ cancel pending position debounce and dock settle
                    ├─ clear the docked hover card
                    └─ persist + emit float://state
```

`AppState::apply_config` deliberately does not copy `float_open` into the
runtime. `save_config` calls `AppConfig::preserve_float_fields` before writing,
so a Settings save with a stale frontend snapshot cannot overwrite
`float_open` / `float_mode` / `float_position`.

Before exiting, `actions::quit` sets `AppState.shutting_down`. An
exit-triggered float close then only clears the runtime state; it does not
persist `float_open = false`, so the next launch can restore the window.

Dock state is in-memory only: after a restart the window reopens in the
configured Full/Compact layout at the saved position, clamped so a rail
position becomes a flush-to-the-edge card. Within one session a close/reopen
keeps the docked presentation and the measured strip length, but always starts
from the collapsed strip: the hover card is cleared on close and on every
window rebuild.

## 7. Testing

Pure logic is tested on every platform:

- `float/layout.rs`: height clamp/card count, fixed sizes, top-strip and side
  rail length clamps, edge distance math (including negative/past-the-edge
  distances and a secondary monitor), docking hysteresis at 1.0/1.5 DPI, top
  priority in corners, pending side undocks, the re-dock guard, nearest-side
  snapping, position clamp, hover-card size, the anchored hover/collapse
  position on every edge (including work-area clamping), radius mapping
  (including the expanded card's 16 px).
- `float/state.rs`: presentation derivation, every dock edge, plan application
  (including pending and guard), strip length change detection, hover
  transitions (expand, dragged anchor, collapse, cleared by un-dock/pending),
  move generations, settle cancellation.
- `config`: legacy `"docked"` loads as Compact; float fields survive a
  settings round-trip.
- `src-tauri/tests/float_state.rs`: DTO/mode contract against `AppState`,
  including `dockHover` (never set while undocked, cleared with the dock).
- `src-tauri/tests/dto_contract.rs`: `presentationMode` names (including
  `docked_right`), `dockHover`, and the removed `topDocked` field.
- `src/windows/float/FloatWindow.test.tsx`: boot shell, every view, both side
  rails, event-driven edge switch, stale initial snapshot, controls,
  empty/error states, long labels, extreme percentages, strip measurement,
  and the hover card (delayed expand request, three rings in canonical order,
  delayed collapse request, re-enter cancelling the collapse, loading state).
- `src/windows/float/floatLayout.test.ts`: the `dockedWindowLength` helper
  (chrome + rounding, unmeasurable content), `dockedEdge` mapping,
  `dockHoverWindows` (canonical order, extras, cap), `ringDashOffset`
  (percent arc, minimum visible arc, clamping), and `snapshotEqual`.

Manual Windows checks (see `docs/tauri-migration/smoke-test.md`): top-most,
drag, top docking hysteresis, side docking on both edges, deferred side
expansion, no immediate re-dock, mode/position/open restore, mixed-DPI rounded
corners, 5+ quota windows, tray checkmark sync, and the docked hover card
(expand/collapse on all three edges, no flicker when the pointer just crosses
the strip, strip position restored after collapsing near a screen corner).
