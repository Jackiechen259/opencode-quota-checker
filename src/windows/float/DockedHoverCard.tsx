// Hover card of the docked floating window.
//
// The docked strip is deliberately a one-line capsule; resting the pointer on
// it expands the same native window (sized by `float::layout::dock_hover_size`)
// into this small overview. The card is the only place where all three quota
// windows are visible at once without opening the Full layout: one ring per
// window, coloured by that window's own health, with the remaining quota under
// it.
//
// The card is rendered ONLY when `float://state` reports `dockHover`; the
// component never sets a hover state of its own. The percentage is the same
// "used" figure as every other view, and the ring arc reuses `progressWidth`
// so a nearly empty window still shows a visible sliver.

import { Dot } from "../../components/common";
import { Icons } from "../../components/icons";
import {
  quotaHealth,
  type FloatDockEdge,
  type UsageReport,
  type WindowReport,
} from "../../types/models";
import {
  DOCK_HOVER_RING_RADIUS,
  DOCK_HOVER_RING_SIZE,
  DOCK_HOVER_RING_STROKE,
  clampPercent,
  compactNumber,
  dockHoverWindows,
  healthColors,
  ringCircumference,
  ringDashOffset,
} from "./floatLayout";
import { RelativeUpdateTime } from "./tickers";

const RING_CENTER = DOCK_HOVER_RING_SIZE / 2;
const RING_CIRCUMFERENCE = ringCircumference();

/** One quota window as a ring gauge with its label and remaining amount. */
function QuotaRing({ window }: { window: WindowReport }) {
  const health = quotaHealth(window.percent, window.limited);
  const accent = healthColors[health];
  const percent = clampPercent(window.percent);
  const remaining = compactNumber(window.remaining);
  return (
    <div
      className="float-dock-ring"
      title={`${window.label}：已用 ${window.percent.toFixed(1)}%，剩余 ${remaining}`}
    >
      <div
        className="float-dock-ring-gauge"
        role="progressbar"
        aria-valuenow={Math.round(percent)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`${window.label} 已用 ${percent.toFixed(1)}%`}
      >
        <svg
          className="float-dock-ring-svg"
          viewBox={`0 0 ${DOCK_HOVER_RING_SIZE} ${DOCK_HOVER_RING_SIZE}`}
          aria-hidden="true"
        >
          <g transform={`rotate(-90 ${RING_CENTER} ${RING_CENTER})`}>
            <circle
              className="float-dock-ring-track"
              cx={RING_CENTER}
              cy={RING_CENTER}
              r={DOCK_HOVER_RING_RADIUS}
              strokeWidth={DOCK_HOVER_RING_STROKE}
            />
            <circle
              className="float-dock-ring-fill"
              cx={RING_CENTER}
              cy={RING_CENTER}
              r={DOCK_HOVER_RING_RADIUS}
              strokeWidth={DOCK_HOVER_RING_STROKE}
              stroke={accent}
              strokeDasharray={RING_CIRCUMFERENCE}
              strokeDashoffset={ringDashOffset(window.percent)}
            />
          </g>
        </svg>
        <span className="float-dock-ring-value" style={{ color: accent }}>
          {window.percent.toFixed(1)}%
        </span>
      </div>
      <span className="float-dock-ring-label" title={window.label}>
        {window.label}
      </span>
      <span className="float-dock-ring-remaining">剩余 {remaining}</span>
    </div>
  );
}

export function DockedHoverCard({
  edge,
  report,
  loading,
  error,
  onClose,
}: {
  /** Edge the window is still snapped to; decides the card's opening side. */
  edge: FloatDockEdge;
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  onClose: () => void;
}) {
  const rings = dockHoverWindows(report);
  let dotColor = "var(--text-tertiary)";
  let stateText = "等待用量数据";
  if (error) {
    dotColor = "var(--danger)";
    stateText = "更新失败";
  } else if (loading) {
    dotColor = "var(--accent)";
    stateText = "正在同步用量…";
  }

  return (
    <div className={`float-dock-card float-dock-card--${edge}`}>
      <div className="float-dock-card-header" data-tauri-drag-region="deep">
        <span className="float-dock-card-title">配额概览</span>
        <span className="float-spacer" />
        {report ? (
          <RelativeUpdateTime
            fetchedMs={report.fetched_at}
            className="float-dock-card-time"
          />
        ) : null}
        <button
          type="button"
          className="float-dock-card-close"
          title="关闭悬浮窗"
          aria-label="关闭悬浮窗"
          onClick={onClose}
        >
          <Icons.Close size={12} />
        </button>
      </div>
      {rings.length > 0 ? (
        <div className="float-dock-rings">
          {rings.map((window) => (
            <QuotaRing key={window.key} window={window} />
          ))}
        </div>
      ) : (
        <div className="float-dock-card-state">
          <Dot color={dotColor} size={8} />
          <span>{stateText}</span>
        </div>
      )}
    </div>
  );
}
