// Explicit 8-direction resize handles for the borderless main window.
//
// Why not rely on `resizable: true` native borders: the window is fully
// undecorated (`decorations: false`) and the visible frame is an app-drawn
// `main-frame` with `overflow:hidden` and rounded corners. Native hit-test
// borders are not observable and are easily clipped by the frame. Explicit
// handles make resize an isolated, testable DOM layer that delegates to the
// OS via `startResizeDragging(direction)` — no JS geometry math.
//
// Layout (per spec Phase 9):
//   - position: fixed, transparent, narrow hit areas (5-8px edges, 8-12px corners)
//   - z-index above page content, below overlays that need special handling
//   - cursor reflects the resize axis
//   - disabled entirely when maximized (Phase 10)

import { useCallback } from "react";
import { windowService, type ResizeDirection } from "../services/window";

interface Props {
  maximized: boolean | null;
}

type HandleDef = {
  direction: ResizeDirection;
  className: string;
  ariaLabel: string;
};

const HANDLES: readonly HandleDef[] = [
  { direction: "North", className: "resize-handle resize-handle-n", ariaLabel: "Resize top" },
  { direction: "South", className: "resize-handle resize-handle-s", ariaLabel: "Resize bottom" },
  { direction: "West", className: "resize-handle resize-handle-w", ariaLabel: "Resize left" },
  { direction: "East", className: "resize-handle resize-handle-e", ariaLabel: "Resize right" },
  { direction: "NorthWest", className: "resize-handle resize-handle-nw", ariaLabel: "Resize top left" },
  { direction: "NorthEast", className: "resize-handle resize-handle-ne", ariaLabel: "Resize top right" },
  { direction: "SouthWest", className: "resize-handle resize-handle-sw", ariaLabel: "Resize bottom left" },
  { direction: "SouthEast", className: "resize-handle resize-handle-se", ariaLabel: "Resize bottom right" },
] as const;

export function WindowResizeHandles({ maximized }: Props) {
  const handlePointerDown = useCallback(
    (direction: ResizeDirection) => (event: React.PointerEvent) => {
      if (maximized) return;
      if (event.button !== 0) return;
      event.preventDefault();
      event.stopPropagation();
      void windowService.startResizeDragging(direction);
    },
    [maximized],
  );

  const handleMouseDown = useCallback(
    (direction: ResizeDirection) => (event: React.MouseEvent) => {
      if (typeof window !== "undefined" && typeof (window as unknown as Record<string, unknown>).PointerEvent !== "undefined") {
        return;
      }
      if (maximized) return;
      if (event.button !== 0) return;
      event.preventDefault();
      event.stopPropagation();
      void windowService.startResizeDragging(direction);
    },
    [maximized],
  );

  if (maximized) return null;

  return (
    <>
      {HANDLES.map((handle) => (
        <div
          key={handle.direction}
          className={handle.className}
          aria-hidden="true"
          onPointerDown={handlePointerDown(handle.direction)}
          onMouseDown={handleMouseDown(handle.direction)}
        />
      ))}
    </>
  );
}

export const RESIZE_HANDLES = HANDLES;
