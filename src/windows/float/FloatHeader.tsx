// Floating window header: brand block (drag region) + window actions.
// Buttons carry no drag-region attribute, so they always stay clickable.

import { Dot } from "../../components/common";
import { Icons } from "../../components/icons";

type HeaderStatus = "loading" | "error" | "ok" | "idle";

function statusOf(loading: boolean, error: string | null, hasData: boolean): HeaderStatus {
  if (loading) return "loading";
  if (error) return "error";
  if (hasData) return "ok";
  return "idle";
}

const statusDot: Record<HeaderStatus, string> = {
  loading: "var(--accent)",
  error: "var(--danger)",
  ok: "var(--success)",
  idle: "var(--text-tertiary)",
};

const statusText: Record<HeaderStatus, string> = {
  loading: "同步中",
  error: "更新失败",
  ok: "监测正常",
  idle: "等待数据",
};

export function FloatHeader({
  variant,
  title,
  status: { loading, error, hasData },
  onRefresh,
  onToggleMode,
  onClose,
}: {
  /** "full" shows the app title + status line; "compact" shows the label. */
  variant: "full" | "compact";
  /** Brand title; the compact view passes the highest-risk window label. */
  title: string;
  status: { loading: boolean; error: string | null; hasData: boolean };
  onRefresh: () => void;
  /** Expands Compact → Full; in Full mode this button is hidden. */
  onToggleMode: () => void;
  onClose: () => void;
}) {
  const status = statusOf(loading, error, hasData);
  const compact = variant === "compact";
  return (
    <div className="float-header">
      <div className="float-brand" data-tauri-drag-region>
        <span className="float-logo">OC</span>
        {compact ? (
          <span className="float-brand-title" data-tauri-drag-region title={title}>
            {title}
          </span>
        ) : (
          <div className="float-brand-text" data-tauri-drag-region>
            <span className="float-brand-title" title={title}>
              {title}
            </span>
            <span className="float-brand-status" data-tauri-drag-region>
              <Dot color={statusDot[status]} size={7} />
              {statusText[status]}
            </span>
          </div>
        )}
      </div>
      <span className="float-spacer" data-tauri-drag-region />
      <button
        type="button"
        className="float-icon-button"
        title="立即刷新"
        aria-label="立即刷新"
        onClick={onRefresh}
      >
        <Icons.Refresh size={15} />
      </button>
      {compact ? (
        <button
          type="button"
          className="float-icon-button"
          title="展开全部配额"
          aria-label="展开全部配额"
          onClick={onToggleMode}
        >
          <Icons.Expand size={15} />
        </button>
      ) : (
        <button
          type="button"
          className="float-icon-button"
          title="切换到精简视图"
          aria-label="切换到精简视图"
          onClick={onToggleMode}
        >
          <Icons.Minimize size={15} />
        </button>
      )}
      <button
        type="button"
        className="float-icon-button float-icon-button--close"
        title="关闭悬浮窗"
        aria-label="关闭悬浮窗"
        onClick={onClose}
      >
        <Icons.Close size={15} />
      </button>
    </div>
  );
}
