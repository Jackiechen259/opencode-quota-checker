// Loading / error / empty states for the floating window.
//
// The error detail is clamped to two lines with ellipsis and offered again
// in a tooltip — a long backend message must never blow up the widget.

import { Dot } from "../../components/common";
import { Icons } from "../../components/icons";

export function FloatStateView({
  loading,
  error,
  onRetry,
  variant = "full",
}: {
  loading: boolean;
  error: string | null;
  onRetry: () => void;
  /** "full" centers a block; "compact" renders a single row. */
  variant?: "full" | "compact";
}) {
  const compact = variant === "compact";
  if (compact) {
    const [color, text] = error
      ? ["var(--danger)", error]
      : loading
        ? ["var(--accent)", "正在同步配额…"]
        : ["var(--text-tertiary)", "暂无配额数据"];
    return (
      <div className="float-state float-state--row">
        <Dot color={color} size={9} />
        <span className="float-state-detail" title={error ?? undefined}>
          {text}
        </span>
        {!loading ? (
          <button type="button" className="float-state-retry" onClick={onRetry}>
            重试
          </button>
        ) : null}
      </div>
    );
  }

  let title: string;
  let detail: string;
  let color: string;
  let showRetry = false;
  if (error) {
    title = "暂时无法更新用量";
    detail = error;
    color = "var(--danger)";
    showRetry = true;
  } else if (loading) {
    title = "正在同步 OpenCode 配额…";
    detail = "首次同步通常只需几秒";
    color = "var(--accent)";
  } else {
    title = "暂无配额数据";
    detail = "点击刷新重新同步";
    color = "var(--text-tertiary)";
    showRetry = true;
  }
  return (
    <div className="float-state">
      <Dot color={color} size={12} />
      <div className="float-state-title">{title}</div>
      <div className="float-state-detail" title={error ?? undefined}>
        {detail}
      </div>
      {showRetry ? (
        <button type="button" className="float-state-retry" onClick={onRetry}>
          <Icons.Refresh size={12} />
          刷新
        </button>
      ) : null}
    </div>
  );
}
