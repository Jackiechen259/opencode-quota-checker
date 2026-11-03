// Docked floating view: a true one-line status strip.
//
// The same markup serves all three dock edges. The top edge lays the strip out
// horizontally; the side edges switch the strip to vertical writing
// (`.float-docked-content--side`), so every glyph — Chinese characters and the
// percentage digits alike — stands upright and reads top-to-bottom. The whole
// strip is a deep drag region, so the label/progress/percent/remaining text are
// draggable as well as the empty areas.
//
// Docking is deliberately read-only apart from closing: there is NO expand
// button. Dragging a top strip away from the screen top un-docks immediately;
// a side rail keeps its rail presentation while the drag is in flight and
// expands once the drag settles (the native move loop owns the window position
// until then), so the card cannot half-appear off-screen.
//
// The native window hugs the rendered content: after every content change the
// component measures an out-of-flow clone along the strip axis and reports the
// strip length to the backend (`set_float_docked_length`), which owns and
// clamps the actual native size (width for the top strip, height for a rail).

import { type CSSProperties, useEffect, useRef } from "react";
import { Dot } from "../../components/common";
import { Icons } from "../../components/icons";
import { percent } from "../../lib/format";
import { api } from "../../services/tauri";
import {
  quotaHealth,
  type FloatDockEdge,
  type UsageReport,
} from "../../types/models";
import {
  clampPercent,
  compactNumber,
  dockedWindowLength,
  healthColors,
  highestWindow,
  progressWidth,
} from "./floatLayout";

/**
 * Natural length of the strip content along its axis.
 *
 * The live content may already be squeezed into the current native length, so
 * an out-of-flow copy is measured: horizontally at `max-content` for the top
 * strip, vertically at `max-content` for a side rail. The clone is appended to
 * `document.body` so it is measured outside the rail's fixed native box; a
 * `vertical-rl` side clone keeps the exact same vertical metrics the rail
 * renders with.
 *
 * Returns 0 when the document has no layout (unit tests);
 * `dockedWindowLength` turns that into `null` so nothing is sent.
 */
function naturalContentLength(content: HTMLElement, vertical: boolean): number {
  const clone = content.cloneNode(true) as HTMLElement;
  clone.style.position = "absolute";
  clone.style.left = "0";
  clone.style.top = "0";
  clone.style.visibility = "hidden";
  clone.style.pointerEvents = "none";
  if (vertical) {
    clone.style.writingMode = "vertical-rl";
    clone.style.width = "24px";
    clone.style.height = "max-content";
  } else {
    clone.style.width = "max-content";
  }
  document.body.appendChild(clone);
  const rect = clone.getBoundingClientRect();
  const length = vertical ? rect.height : rect.width;
  clone.remove();
  return length;
}

export function DockedView({
  edge,
  report,
  loading,
  error,
  onClose,
}: {
  /** Edge the window is snapped to; decides the horizontal/vertical strip. */
  edge: FloatDockEdge;
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  onClose: () => void;
}) {
  const contentRef = useRef<HTMLDivElement>(null);
  const vertical = edge !== "top";

  useEffect(() => {
    const content = contentRef.current;
    if (!content) return;
    const length = dockedWindowLength(naturalContentLength(content, vertical));
    if (length !== null) void api.setFloatDockedLength(length);
  }, [report, loading, error, vertical]);

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
    const used = clampPercent(window.percent);
    const fill = progressWidth(window.percent);
    // The meter is physical: full width and a partial height on a vertical
    // rail, full height and a partial width on the horizontal strip.
    const fillStyle: CSSProperties = vertical
      ? { width: "100%", height: `${fill}%`, background: accent }
      : { width: `${fill}%`, height: "100%", background: accent };
    content = (
      <>
        <Dot color={accent} size={8} />
        <span className="float-docked-label" title={window.label}>
          {window.label}
        </span>
        <span
          className="float-docked-progress"
          role="progressbar"
          aria-valuenow={Math.round(used)}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label={`${window.label} 已用 ${percent(window.percent)}`}
        >
          <span className="float-docked-progress-fill" style={fillStyle} />
        </span>
        <span className="float-docked-percent" style={{ color: accent }}>
          {percent(window.percent)}
        </span>
        <span className="float-docked-remaining">
          剩余 {compactNumber(window.remaining)}
        </span>
      </>
    );
  }
  return (
    <div
      className={`float-docked float-docked--${edge}`}
      data-tauri-drag-region="deep"
    >
      <div
        className={`float-docked-content${vertical ? " float-docked-content--side" : ""}`}
        ref={contentRef}
      >
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
