// Floating window: Full / Compact / Docked modes rendered from the shared
// quota state. The window itself (size, position, docking, rounded corners)
// is owned by the Rust backend; this component only renders and sends
// commands. The whole docked strip is a drag region.

import { useCallback, useEffect, useState, type ReactNode } from "react";
import { Dot } from "../components/common";
import { Icons } from "../components/icons";
import { useQuota } from "../hooks/useApp";
import { useNow, useTauriEvent } from "../hooks/useTauriEvents";
import { countdownShort, number } from "../lib/format";
import { api } from "../services/tauri";
import { EVENT, quotaHealth, type FloatMode, type FloatStateDto, type UsageReport, type WindowReport } from "../types/models";

function highest(report: UsageReport | null): WindowReport | null {
  if (!report) return null;
  let best: WindowReport | null = null;
  for (const window of report.windows) {
    if (!best || window.percent > best.percent) best = window;
  }
  return best;
}

const healthColors: Record<"healthy" | "warning" | "critical", string> = {
  healthy: "var(--color-success)",
  warning: "var(--color-warning)",
  critical: "var(--color-danger)",
};

function healthLabel(health: "healthy" | "warning" | "critical"): string {
  return health === "healthy" ? "状态健康" : health === "warning" ? "接近阈值" : "已达危险阈值";
}

export function FloatWindow() {
  const { report, loading, error, reload } = useQuota();
  const nowMs = useNow();
  const [floatState, setFloatState] = useState<FloatStateDto | null>(null);
  const [mode, setMode] = useState<FloatMode>("full");

  useTauriEvent<FloatStateDto>(EVENT.FLOAT_STATE, (payload) => {
    setFloatState(payload);
    setMode(payload.top_docked ? "docked" : payload.mode);
  });

  useEffect(() => {
    void api.getFloatState().then((state) => {
      setFloatState(state);
      setMode(state.top_docked ? "docked" : state.mode);
    });
    void reload();
  }, [reload]);

  // Re-poll the float state after an interval so top-docking triggered by
  // native drags (which emit no command) still lands in the UI.
  useEffect(() => {
    const timer = window.setInterval(() => {
      void api.getFloatState().then((state) => {
        setFloatState(state);
        setMode(state.top_docked ? "docked" : state.mode);
      });
    }, 1_000);
    return () => window.clearInterval(timer);
  }, []);

  const changeMode = useCallback((next: FloatMode) => {
    void api.setFloatMode(next);
  }, []);

  const close = useCallback(() => {
    void api.closeFloatWindow();
  }, []);

  const refresh = useCallback(() => {
    void api.refreshUsage();
  }, []);

  const effectiveMode = floatState?.top_docked ? "docked" : mode;
  const windowClass = `float-window float-${effectiveMode}`;
  const dotColor = loading
    ? "var(--color-primary)"
    : error
      ? "var(--color-danger)"
      : report
        ? "var(--color-success)"
        : "var(--color-text-muted)";
  const statusText = loading
    ? "同步中"
    : error
      ? "更新失败"
      : report
        ? "监测正常"
        : "等待数据";

  return (
    <div className={windowClass} data-tauri-drag-region>
      {effectiveMode === "docked" ? (
        <DockedView
          report={report}
          loading={loading}
          onExpand={() => changeMode("full")}
          onClose={close}
        />
      ) : (
        <div className="float-inner" data-tauri-drag-region>
          <div className="float-header" data-tauri-drag-region>
            <div className="float-brand" data-tauri-drag-region>
              <span className="float-logo">OC</span>
              <div className="float-brand-text">
                <span className="float-brand-title">OpenCode Quota Checker</span>
                <span className="float-brand-status">
                  <Dot color={dotColor} size={8} />
                  {statusText}
                </span>
              </div>
            </div>
            <span style={{ flex: 1 }} />
            <button type="button" className="float-icon-button" title="立即刷新" aria-label="立即刷新" onClick={refresh}>
              <Icons.Refresh size={13} />
            </button>
            {effectiveMode === "full" ? (
              <button type="button" className="float-icon-button" title="切换到精简视图" aria-label="切换到精简视图" onClick={() => changeMode("compact")}>
                <Icons.Minimize size={13} />
              </button>
            ) : (
              <button type="button" className="float-icon-button" title="展开全部配额" aria-label="展开全部配额" onClick={() => changeMode("full")}>
                <Icons.Expand size={13} />
              </button>
            )}
            <button type="button" className="float-icon-button" title="关闭悬浮窗" aria-label="关闭悬浮窗" onClick={close}>
              <Icons.Close size={13} />
            </button>
          </div>

          {effectiveMode === "full" ? (
            <FullView report={report} loading={loading} error={error?.user ?? null} nowMs={nowMs} />
          ) : (
            <CompactView report={report} loading={loading} error={error?.user ?? null} nowMs={nowMs} />
          )}
        </div>
      )}
    </div>
  );
}

function FullView({
  report,
  loading,
  error,
  nowMs,
}: {
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  nowMs: number;
}) {
  if (!report) {
    const [title, detail, color] = error
      ? ["暂时无法更新用量", error, "var(--color-danger)"]
      : loading
        ? ["正在获取用量", "首次同步通常只需几秒", "var(--color-primary)"]
        : ["还没有用量数据", "点击右上角刷新按钮开始同步", "var(--color-text-muted)"];
    return (
      <div className="float-empty">
        <Dot color={color} size={13} />
        <div className="float-empty-title">{title}</div>
        <div className="float-empty-detail">{detail}</div>
      </div>
    );
  }
  const highestWindow = highest(report);
  const plan = report.plan_type.trim() ? `OpenCode Go · ${report.plan_type}` : "OpenCode Go";
  return (
    <div className="float-full">
      <div className="float-meta">
        <span className="float-plan-badge">{plan}</span>
        <span className="float-meta-count">共 {report.windows.length} 个配额周期</span>
        <span style={{ flex: 1 }} />
        <span className="float-meta-highest" style={{ color: healthColors[quotaHealth(highestWindow?.percent ?? 0)] }}>
          最高 {highestWindow ? highestWindow.percent.toFixed(1) : "0.0"}%
        </span>
      </div>
      {report.windows.map((window) => (
        <FloatWindowCard key={window.key} window={window} nowMs={nowMs} />
      ))}
      <div className="float-footer-row">
        <span>自动监测中</span>
        <span style={{ flex: 1 }} />
        <span>更新于 {relativeTime(report.fetched_at, nowMs)}</span>
      </div>
    </div>
  );
}

function CompactView({
  report,
  loading,
  error,
  nowMs,
}: {
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  nowMs: number;
}) {
  const window = highest(report);
  if (!window) {
    const detail = error ? error : loading ? "首次同步通常只需几秒" : "点击右上角刷新按钮开始同步";
    return (
      <div className="float-empty float-empty-small">
        <Dot color={error ? "var(--color-danger)" : loading ? "var(--color-primary)" : "var(--color-text-muted)"} size={10} />
        <span className="float-empty-detail">{detail}</span>
      </div>
    );
  }
  const health = quotaHealth(window.percent);
  const color = healthColors[health];
  const resetSeconds = Math.max(0, Math.floor((window.reset_time - nowMs) / 1_000));
  return (
    <div className="float-compact">
      <div className="float-compact-top">
        <div className="float-compact-label-block">
          <span className="float-compact-label">{window.label}</span>
          <span className="float-compact-percent" style={{ color }}>
            {window.percent.toFixed(1)}%
          </span>
        </div>
        <div className="float-compact-remaining">
          <span className="float-compact-remaining-label">剩余额度</span>
          <span className="float-compact-remaining-value">{number(window.remaining)}</span>
        </div>
      </div>
      <div className="progress float-compact-progress">
        <div
          className="progress-fill"
          style={{ width: `${Math.min(100, Math.max(0, window.percent))}%`, background: color, height: 7 }}
        />
      </div>
      <div className="float-compact-bottom">
        <span className="float-compact-health" style={{ color }}>
          {healthLabel(health)}
        </span>
        <span style={{ flex: 1 }} />
        <span className="float-compact-reset">重置后 {countdownShort(resetSeconds)}</span>
      </div>
    </div>
  );
}

function FloatWindowCard({ window, nowMs }: { window: WindowReport; nowMs: number }) {
  const health = quotaHealth(window.percent);
  const accent = healthColors[health];
  const resetSeconds = Math.max(0, Math.floor((window.reset_time - nowMs) / 1_000));
  return (
    <div className="float-quota-panel" style={{ borderColor: accent }}>
      <div className="float-panel-row">
        <span className="float-panel-title-row">
          <Dot color={accent} size={9} />
          {window.label}
        </span>
        <span className="float-panel-percent" style={{ color: accent }}>
          {window.percent.toFixed(1)}%
        </span>
      </div>
      <div className="progress float-panel-progress">
        <div
          className="progress-fill"
          style={{ width: `${Math.min(100, Math.max(0, window.percent))}%`, background: accent, height: 6 }}
        />
      </div>
      <div className="float-panel-row">
        <span className="float-panel-muted">
          已用 {number(window.used)} / {number(window.quota)}
        </span>
        <span className="float-panel-muted">{countdownShort(resetSeconds)} 后重置</span>
      </div>
    </div>
  );
}

function DockedView({
  report,
  loading,
  onExpand,
  onClose,
}: {
  report: UsageReport | null;
  loading: boolean;
  onExpand: () => void;
  onClose: () => void;
}) {
  const window = highest(report);
  let content: ReactNode;
  if (!window) {
    content = (
      <span className="float-docked-status">
        <Dot color="var(--color-text-muted)" size={9} />
        {loading ? "正在同步用量…" : "等待用量数据"}
      </span>
    );
  } else {
    const health = quotaHealth(window.percent);
    const color = healthColors[health];
    const remaining = number(window.remaining);
    content = (
      <span className="float-docked-main">
        <Dot color={color} size={9} />
        <span className="float-docked-label">{window.label}</span>
        <span className="float-docked-percent" style={{ color }}>
          {window.percent.toFixed(1)}%
        </span>
        <span className="float-docked-remaining">余 {remaining}</span>
      </span>
    );
  }
  return (
    <div className="float-docked" data-tauri-drag-region>
      <span className="float-docked-content" data-tauri-drag-region>
        {content}
      </span>
      <button type="button" className="float-docked-button" title="展开" aria-label="展开" onClick={onExpand}>
        展开
      </button>
      <button type="button" className="float-docked-button float-docked-close" title="关闭" aria-label="关闭" onClick={onClose}>
        ×
      </button>
    </div>
  );
}

function relativeTime(fetchedAtMs: number, nowMs: number): string {
  const delta = Math.max(0, Math.floor((nowMs - fetchedAtMs) / 1_000));
  if (delta < 5) return "刚刚";
  if (delta < 60) return `${delta} 秒前`;
  const minutes = Math.floor(delta / 60);
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  return `${Math.floor(hours / 24)} 天前`;
}
