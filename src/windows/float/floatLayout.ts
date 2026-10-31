// Pure layout/presentation helpers for the floating window.
//
// Everything here is deterministic and unit-testable; components stay thin
// wrappers around these functions.

import type {
  FloatDockEdge,
  FloatMode,
  FloatStateDto,
  QuotaHealth,
  UsageReport,
  WindowReport,
} from "../../types/models";

/** Minimum fill width of a progress bar, as a percentage of the track.
 *  A 0.1% quota must stay visible instead of vanishing into the track. */
export const MIN_FILL_PERCENT = 1.2;

/** Health accent colors; only dot / percent / progress use them. */
export const healthColors: Record<QuotaHealth, string> = {
  healthy: "var(--success)",
  warning: "var(--warning)",
  critical: "var(--danger)",
};

export function healthLabel(health: QuotaHealth): string {
  return health === "healthy" ? "状态健康" : health === "warning" ? "接近阈值" : "已达危险阈值";
}

/** The highest-risk quota window (used by Compact/Docked). */
export function highestWindow(report: UsageReport | null): WindowReport | null {
  if (!report) return null;
  let best: WindowReport | null = null;
  for (const window of report.windows) {
    if (!best || window.percent > best.percent) best = window;
  }
  return best;
}

/**
 * Progress fill width in percent, clamped to 0..100 with a minimum visible
 * width for any positive usage (avoiding an invisible sliver at 0.1%).
 */
export function progressWidth(percent: number): number {
  if (percent <= 0) return 0;
  return Math.min(100, Math.max(MIN_FILL_PERCENT, percent));
}

/** Clamped 0..100 percent for aria values. */
export function clampPercent(percent: number): number {
  return Math.min(100, Math.max(0, percent));
}

/** Horizontal chrome around the docked content, in CSS px.
 *
 * 8 px padding × 2 + 1 px outline × 2 + two 4 px flex gaps + the 20 px close
 * button. This is the strip length (width for the top strip, height for a side
 * rail) that the backend adds to the measured content. Keep in sync with
 * `.float-docked` / `.float-docked-close` in `float-window.css`; the Rust
 * controller clamps the total to the presentation's legal range. */
export const DOCKED_CHROME_LENGTH = 46;

/** Extra pixels so a fractional measurement cannot clip the close button. */
const DOCKED_LENGTH_SLACK = 2;

/**
 * Native window length that hugs the measured docked content.
 *
 * The length runs along the strip axis: the top strip's width, a side rail's
 * height. Returns `null` when the DOM has no layout (unit tests, hidden
 * window), so callers never send a meaningless 0 px length to the backend.
 */
export function dockedWindowLength(contentLength: number): number | null {
  if (!Number.isFinite(contentLength) || contentLength <= 0) return null;
  return Math.ceil(contentLength) + DOCKED_CHROME_LENGTH + DOCKED_LENGTH_SLACK;
}

/**
 * Edge a presentation is docked to, or `null` for the Full/Compact layouts.
 *
 * The backend encodes the edge in `presentationMode`, so the view needs no
 * second source of truth to decide between the horizontal strip and the
 * vertical rails.
 */
export function dockedEdge(mode: FloatMode): FloatDockEdge | null {
  switch (mode) {
    case "docked_top":
      return "top";
    case "docked_left":
      return "left";
    case "docked_right":
      return "right";
    default:
      return null;
  }
}

/** Whole-number remaining quota ("960", "1,000"). */
export function compactNumber(value: number): string {
  return Math.round(value).toLocaleString("en-US");
}

/** Number of quota rings the docked hover card shows. */
export const DOCK_HOVER_RING_COUNT = 3;

/**
 * Quota windows the hover card prefers, in ring order.
 *
 * OpenCode Go reports exactly these three windows; naming them keeps the card
 * stable when a partial response omits one.
 */
const DOCK_HOVER_KEYS = ["rolling-5h", "weekly", "monthly"] as const;

/**
 * The (up to three) quota windows the docked hover card shows.
 *
 * The card is a three-ring overview: the OpenCode windows come first in their
 * canonical 5-hour / weekly / monthly order, then any extra or differently
 * keyed window fills the remaining ring in report order. Missing windows are
 * simply omitted (the rings stay centered) instead of reserving an empty slot,
 * and a fourth window never pushes the canonical three out of the card.
 */
export function dockHoverWindows(report: UsageReport | null): WindowReport[] {
  if (!report) return [];
  const byKey = new Map(report.windows.map((window) => [window.key, window]));
  const picked: WindowReport[] = [];
  const pickedKeys = new Set<string>();
  for (const key of DOCK_HOVER_KEYS) {
    const window = byKey.get(key);
    if (!window) continue;
    picked.push(window);
    pickedKeys.add(window.key);
  }
  for (const window of report.windows) {
    if (picked.length >= DOCK_HOVER_RING_COUNT) break;
    if (pickedKeys.has(window.key)) continue;
    picked.push(window);
    pickedKeys.add(window.key);
  }
  return picked;
}

/** Side of the square viewBox one hover-card ring is drawn in. */
export const DOCK_HOVER_RING_SIZE = 84;
/** Stroke width of the ring, in the same user units. */
export const DOCK_HOVER_RING_STROKE = 7;
/** Radius of the stroke's centre line, so the stroke fits the viewBox. */
export const DOCK_HOVER_RING_RADIUS = (DOCK_HOVER_RING_SIZE - DOCK_HOVER_RING_STROKE) / 2;

/** Full circumference of a ring's progress arc, in user units. */
export function ringCircumference(radius = DOCK_HOVER_RING_RADIUS): number {
  return 2 * Math.PI * radius;
}

/**
 * `stroke-dashoffset` that draws `percent` of a ring.
 *
 * Reuses `progressWidth`, so the same minimum-visible rule as the linear
 * meters applies: a 0.1% window keeps a dot instead of vanishing. The offset
 * is clamped to the circumference, so an out-of-range percent cannot wrap the
 * arc the wrong way.
 */
export function ringDashOffset(percent: number, radius = DOCK_HOVER_RING_RADIUS): number {
  const circumference = ringCircumference(radius);
  const visible = (progressWidth(percent) / 100) * circumference;
  return circumference - Math.min(circumference, visible);
}

/** Seconds until a reset timestamp, never negative. */
export function resetSeconds(resetMs: number, nowMs: number): number {
  return Math.max(0, Math.floor((resetMs - nowMs) / 1_000));
}

/** True when two float snapshots are identical (fallback-poll guard). */
export function snapshotEqual(a: FloatStateDto | null, b: FloatStateDto | null): boolean {
  if (a === b) return true;
  if (!a || !b) return false;
  return (
    a.open === b.open &&
    a.configuredMode === b.configuredMode &&
    a.presentationMode === b.presentationMode &&
    a.dockHover === b.dockHover
  );
}
