// Application-drawn title bar of the borderless main window.
//
// Layout, from left to right: app icon, app name, a draggable spacer, then
// the minimize / maximize-restore / close window controls — the same
// dimensions as a native Windows caption (32 px strip, 46 px buttons).
//
// Drag regions: only `titlebar-brand` and `titlebar-spacer` are
// `data-tauri-drag-region`. The controls container is deliberately NOT a
// drag region so window-control hit testing is structurally impossible to
// confuse with dragging.
//
// Window dragging and controls are isolated at the DOM level:
//
//   titlebar
//   ├── draggable brand  → startDragging() on primary pointerdown
//   ├── draggable spacer → startDragging() on primary pointerdown
//   └── non-draggable controls
//       ├── minimize  (pointerdown stopPropagation)
//       ├── maximize  (pointerdown stopPropagation)
//       └── close     (pointerdown stopPropagation)
//
// Explicit `startDragging()` is used instead of relying solely on
// `data-tauri-drag-region` so the drag path is observable (diagnostics +
// [window-hit] dev logs) and testable.

import { useCallback } from "react";
import { AppLogo } from "./common";
import { Icons } from "./icons";
import { windowService } from "../services/window";

interface Props {
  maximized: boolean | null;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const DEV = (import.meta as any).env?.DEV ?? false;

export function isInteractiveTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return Boolean(target.closest("button, input, select, textarea, a, [role='button']"));
}

function logHit(message: string) {
  if (DEV) console.debug(message);
}

export function TitleBar({ maximized }: Props) {
  const toggleMaximize = useCallback(() => {
    void windowService.toggleMaximize();
    // The real state arrives through the shared resize/move listener
    // (debounced in `useWindowState`); no local re-query is needed.
  }, []);

  const handleDragPointerDown = useCallback(
    (event: React.PointerEvent) => {
      if (event.button !== 0) return;
      if (isInteractiveTarget(event.target)) return;
      // Single click + hold starts a native drag. Double-click is handled
      // separately via onDoubleClick to toggle maximize.
      // `detail` is 1 for a single click; synthetic test events may carry 0
      // when `detail` is not supplied, so only suppress >1 (double-click).
      if (event.detail > 1) return;
      logHit("[window-hit] titlebar pointerdown");
      event.preventDefault();
      void windowService.startDragging();
    },
    [],
  );

  // Fallback for environments without PointerEvent; ignored when
  // PointerEvent exists so the pointer handler remains the single source.
  const handleDragMouseDown = useCallback(
    (event: React.MouseEvent) => {
      if (typeof window !== "undefined" && typeof (window as unknown as Record<string, unknown>).PointerEvent !== "undefined") {
        // Real browsers with PointerEvent will already have fired pointerdown.
        return;
      }
      if (event.button !== 0) return;
      if (isInteractiveTarget(event.target)) return;
      if (event.detail > 1) return;
      logHit("[window-hit] titlebar pointerdown");
      event.preventDefault();
      void windowService.startDragging();
    },
    [],
  );

  const handleDragDoubleClick = useCallback(() => {
    logHit("[window-hit] titlebar doubleclick");
    void windowService.toggleMaximize();
  }, []);

  const stopDragPropagation = useCallback((event: React.SyntheticEvent) => {
    event.stopPropagation();
  }, []);

  return (
    <div className="titlebar">
      <div
        className="titlebar-brand"
        data-tauri-drag-region
        onPointerDown={handleDragPointerDown}
        onMouseDown={handleDragMouseDown}
        onDoubleClick={handleDragDoubleClick}
      >
        <AppLogo size={18} />
        <span className="titlebar-title">OpenCode Quota Checker</span>
      </div>
      <div
        className="titlebar-spacer"
        data-tauri-drag-region
        onPointerDown={handleDragPointerDown}
        onMouseDown={handleDragMouseDown}
        onDoubleClick={handleDragDoubleClick}
      />
      <div className="titlebar-controls">
        <button
          type="button"
          className="titlebar-control"
          title="最小化"
          aria-label="最小化"
          onPointerDown={(event) => {
            logHit("[window-hit] minimize pointerdown");
            event.stopPropagation();
          }}
          onMouseDown={stopDragPropagation}
          onClick={() => {
            logHit("[window-hit] minimize click");
            void windowService.minimize();
          }}
        >
          <Icons.WinMinimize size={11} />
        </button>
        <button
          type="button"
          className="titlebar-control"
          title={maximized ? "还原" : "最大化"}
          aria-label={maximized ? "还原" : "最大化"}
          onPointerDown={(event) => {
            logHit("[window-hit] maximize pointerdown");
            event.stopPropagation();
          }}
          onMouseDown={stopDragPropagation}
          onClick={() => {
            logHit("[window-hit] maximize click");
            toggleMaximize();
          }}
        >
          {maximized ? <Icons.WinRestore size={11} /> : <Icons.WinMaximize size={11} />}
        </button>
        <button
          type="button"
          className="titlebar-control titlebar-control-close"
          title="关闭"
          aria-label="关闭"
          // Routes through the Rust close handler, which always terminates
          // the process. Hiding to the tray is an explicit action only
          // (header menu "隐藏主窗口" / tray menu).
          onPointerDown={(event) => {
            logHit("[window-hit] close pointerdown");
            event.stopPropagation();
          }}
          onMouseDown={stopDragPropagation}
          onClick={() => {
            logHit("[window-hit] close click");
            void windowService.close();
          }}
        >
          <Icons.WinClose size={11} />
        </button>
      </div>
    </div>
  );
}
