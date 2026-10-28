// Docked floating view: a true one-line status strip.
//
// The whole strip is a deep drag region, so the label/progress/percent/remaining
// text are draggable as well as the empty areas. Docking is deliberately
// read-only apart from closing: there is NO expand button — dragging the strip
// away from the screen top un-docks it and restores the configured
// Full/Compact layout. The close button is the only clickable element and is
// excluded from Tauri's drag script automatically.
//
// The native window hugs the rendered content: after every content change the
// component measures an out-of-flow clone at its natural width and reports the
// total window width to the backend (`set_float_docked_width`), which owns and
// clamps the actual native size.

import { useEffect, useRef } from "react";
import { Dot } from "../../components/common";
import { Icons } from "../../components/icons";
import { api } from "../../services/tauri";
import { quotaHealth, type UsageReport } from "../../types/models";
import {
  clampPercent,
  compactNumber,
  dockedWindowWidth,
  healthColors,
  highestWindow,
  progressWidth,
} from "./floatLayout";

/**
 * Natural width of the strip content.
 *
 * The live content may already be squeezed into the current native width, so
 * an out-of-flow copy is laid out at `max-content` and measured. It returns 0
 * when the document has no layout (unit tests); `dockedWindowWidth` turns that
 * into `null` so nothing is sent.
 */
function naturalContentWidth(content: HTMLElement): number {
  const clone = content.cloneNode(true) as HTMLElement;
  clone.style.position = "absolute";
  clone.style.left = "0";
  clone.style.top = "0";
  clone.style.width = "max-content";
  clone.style.visibility = "hidden";
  clone.style.pointerEvents = "none";
  content.parentElement?.appendChild(clone);
  const width = clone.getBoundingClientRect().width;
  clone.remove();
  return width;
}

export function DockedView({
  report,
  loading,
  error,
  onClose,
}: {
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  onClose: () => void;
}) {
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const content = contentRef.current;
    if (!content) return;
    const width = dockedWindowWidth(naturalContentWidth(content));
    if (width !== null) void api.setFloatDockedWidth(width);
  }, [report, loading, error]);

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
    const percent = clampPercent(window.percent);
    content = (
      <>
        <Dot color={accent} size={8} />
        <span className="float-docked-label" title={window.label}>
          {window.label}
        </span>
        <span
          className="float-docked-progress"
          role="progressbar"
          aria-valuenow={Math.round(percent)}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label={`${window.label} 已用 ${percent.toFixed(1)}%`}
        >
          <span
            className="float-docked-progress-fill"
            style={{ width: `${progressWidth(window.percent)}%`, background: accent }}
          />
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
    <div className="float-docked" data-tauri-drag-region="deep">
      <div className="float-docked-content" ref={contentRef}>
        {content}
      </div>
      <span className="float-spacer" />
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
