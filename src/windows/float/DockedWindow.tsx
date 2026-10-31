// Docked window shell: one native window for the strip and its hover card.
//
// Resting the pointer on the docked strip asks the backend to expand the
// native window into a slightly larger card with one ring per quota window;
// leaving the window asks it to collapse again. Both directions are delayed
// ("hover intent"), so a cursor merely crossing the 24 px strip cannot flash
// the card, and a leave/enter flicker is cancelled by the enter handler.
//
// This component owns no layout state: it renders the card only while
// `float://state` reports `dockHover`, exactly like `presentationMode`. The
// requests are advisory and idempotent — the backend ignores them while the
// window is not docked, while a side un-dock is pending, or while the user is
// still dragging, and it owns the native size in every case.

import { type PointerEvent as ReactPointerEvent, useCallback, useEffect, useRef } from "react";
import { api } from "../../services/tauri";
import type { FloatDockEdge, UsageReport } from "../../types/models";
import { DockedHoverCard } from "./DockedHoverCard";
import { DockedView } from "./DockedView";

/** Resting time before the strip expands; filters out fly-bys. */
const EXPAND_DELAY_MS = 140;
/** Grace period before collapsing; a quick re-enter cancels it. */
const COLLAPSE_DELAY_MS = 260;

export function DockedWindow({
  edge,
  dockHover,
  report,
  loading,
  error,
  onClose,
}: {
  /** Edge the window is snapped to; decides strip orientation and card side. */
  edge: FloatDockEdge;
  /** Backend-owned hover state: the native window is already the card. */
  dockHover: boolean;
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  onClose: () => void;
}) {
  const expandTimer = useRef<number | null>(null);
  const collapseTimer = useRef<number | null>(null);

  const clearTimers = useCallback(() => {
    if (expandTimer.current !== null) window.clearTimeout(expandTimer.current);
    if (collapseTimer.current !== null) window.clearTimeout(collapseTimer.current);
    expandTimer.current = null;
    collapseTimer.current = null;
  }, []);

  // A docked view unmounting mid-delay must not fire a stale resize request.
  useEffect(() => clearTimers, [clearTimers]);

  const request = useCallback((hover: boolean) => {
    void api.setFloatDockHover(hover).catch((cause) => {
      console.error("[float] set_float_dock_hover failed", cause);
    });
  }, []);

  const onPointerEnter = useCallback(() => {
    clearTimers();
    expandTimer.current = window.setTimeout(() => {
      expandTimer.current = null;
      request(true);
    }, EXPAND_DELAY_MS);
  }, [clearTimers, request]);

  const onPointerLeave = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      // A held button means a drag (the strip is a drag region) or a selection
      // is in flight. Collapsing then would resize the window under the native
      // move loop; the docking decision owns that transition instead.
      if ((event.buttons ?? 0) !== 0) return;
      clearTimers();
      collapseTimer.current = window.setTimeout(() => {
        collapseTimer.current = null;
        request(false);
      }, COLLAPSE_DELAY_MS);
    },
    [clearTimers, request],
  );

  return (
    <div
      className={`float-window float-window--docked${dockHover ? " float-window--dock-hover" : ""}`}
      onPointerEnter={onPointerEnter}
      onPointerLeave={onPointerLeave}
    >
      {dockHover ? (
        <DockedHoverCard
          edge={edge}
          report={report}
          loading={loading}
          error={error}
          onClose={onClose}
        />
      ) : (
        <DockedView
          edge={edge}
          report={report}
          loading={loading}
          error={error}
          onClose={onClose}
        />
      )}
    </div>
  );
}
