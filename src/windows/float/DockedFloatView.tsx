// Docked floating view: a true one-line status strip.
//
// The whole content area is a drag region (bare attribute = direct clicks
// only); the buttons carry no drag-region attribute so they stay clickable.

import { Dot } from "../../components/common";
import { Icons } from "../../components/icons";
import { quotaHealth, type UsageReport } from "../../types/models";
import { compactNumber, healthColors, highestWindow } from "./floatLayout";

export function DockedFloatView({
  report,
  loading,
  error,
  onExpand,
  onClose,
}: {
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  onExpand: () => void;
  onClose: () => void;
}) {
  const window = highestWindow(report);
  let content;
  if (!window) {
    const color = error
      ? "var(--danger)"
      : loading
        ? "var(--accent)"
        : "var(--text-tertiary)";
    const text = error ? "更新失败" : loading ? "正在同步用量…" : "等待用量数据";
    content = (
      <span className="float-docked-status">
        <Dot color={color} size={8} />
        {text}
      </span>
    );
  } else {
    const health = quotaHealth(window.percent);
    const accent = healthColors[health];
    content = (
      <>
        <Dot color={accent} size={8} />
        <span className="float-docked-label" title={window.label}>
          {window.label}
        </span>
        <span className="float-docked-percent" style={{ color: accent }}>
          {window.percent.toFixed(1)}%
        </span>
        <span className="float-docked-remaining">
          剩余 {compactNumber(window.remaining)}
        </span>
      </>
    );
  }
  return (
    <div className="float-docked">
      <div className="float-docked-content" data-tauri-drag-region>
        {content}
      </div>
      <span className="float-spacer" data-tauri-drag-region />
      <button
        type="button"
        className="float-docked-button"
        title="展开全部配额"
        aria-label="展开全部配额"
        onClick={onExpand}
      >
        展开
      </button>
      <button
        type="button"
        className="float-docked-button float-docked-close"
        title="关闭悬浮窗"
        aria-label="关闭悬浮窗"
        onClick={onClose}
      >
        <Icons.Close size={12} />
      </button>
    </div>
  );
}
