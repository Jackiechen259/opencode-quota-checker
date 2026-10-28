// Pure layout/presentation helpers for the floating window.
//
// Everything here is deterministic and unit-testable; components stay thin
// wrappers around these functions.

import type { FloatStateDto, QuotaHealth, UsageReport, WindowReport } from "../../types/models";

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

/** Horizontal chrome around the Docked content, in CSS px.
 *
 * 8 px padding × 2 + 1 px outline × 2 + two 4 px flex gaps + the 20 px close
 * button. Keep in sync with `.float-docked` / `.float-docked-close` in
 * `float-window.css`; the Rust controller clamps the total to 96..392. */
export const DOCKED_CHROME_WIDTH = 46;

/** Extra pixels so a fractional measurement cannot clip the close button. */
const DOCKED_WIDTH_SLACK = 2;

/**
 * Native window width that hugs the measured Docked content.
 *
 * Returns `null` when the DOM has no layout (unit tests, hidden window), so
 * callers never send a meaningless 0 px width to the backend.
 */
export function dockedWindowWidth(contentWidth: number): number | null {
  if (!Number.isFinite(contentWidth) || contentWidth <= 0) return null;
  return Math.ceil(contentWidth) + DOCKED_CHROME_WIDTH + DOCKED_WIDTH_SLACK;
}

/** Whole-number remaining quota ("960", "1,000"). */
export function compactNumber(value: number): string {
  return Math.round(value).toLocaleString("en-US");
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
    a.topDocked === b.topDocked
  );
}
