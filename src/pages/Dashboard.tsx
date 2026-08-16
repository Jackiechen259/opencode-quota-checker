// Dashboard page: overview hero + quota window grid.
//
// Responsive layout is CSS-driven (`.quota-grid` auto-fit columns); no JS
// resize listeners or `window.innerWidth` reads.

import { useMemo } from "react";
import { DetailsHeader, OverviewCard, QuotaCard } from "../components/quota";
import { Card, EmptyState, Notice } from "../components/common";
import type { AppError, UsageReport } from "../types/models";
import { Icons } from "../components/icons";

interface Props {
  report: UsageReport | null;
  loading: boolean;
  error: AppError | null;
  nowMs: number;
  onRefresh: () => void;
}

/** Skeleton placeholders mirroring the real dashboard structure: hero card,
 *  three quota cards, then a caption. Decorative blocks are aria-hidden. */
function DashboardSkeleton() {
  return (
    <div className="dashboard-skeleton" aria-busy="true" aria-label="正在加载用量数据">
      <div className="card skeleton-card">
        <div className="skeleton-row">
          <span className="sk sk-dot" aria-hidden="true" />
          <span className="sk sk-line" style={{ width: 110 }} aria-hidden="true" />
          <span style={{ flex: 1 }} />
          <span className="sk sk-line" style={{ width: 84, height: 24, borderRadius: 999 }} aria-hidden="true" />
        </div>
        <div className="skeleton-row">
          <span className="sk sk-hero" style={{ width: 210 }} aria-hidden="true" />
          <span style={{ flex: 1 }} />
          <span className="sk sk-line" style={{ width: 120, height: 18 }} aria-hidden="true" />
        </div>
        <span className="sk sk-bar" aria-hidden="true" />
        <div className="sk-stats" aria-hidden="true">
          <span className="sk sk-tile" />
          <span className="sk sk-tile" />
          <span className="sk sk-tile" />
          <span className="sk sk-tile" />
        </div>
      </div>
      <div className="quota-grid" aria-hidden="true">
        {[0, 1, 2].map((index) => (
          <div className="card skeleton-card" key={index}>
            <div className="skeleton-row">
              <span className="sk sk-dot" />
              <span className="sk sk-line" style={{ width: 90 }} />
              <span style={{ flex: 1 }} />
              <span className="sk sk-line" style={{ width: 60, height: 24, borderRadius: 999 }} />
            </div>
            <div className="skeleton-row">
              <span className="sk sk-ring" />
              <div className="sk-metrics">
                <span className="sk sk-line" style={{ width: "100%" }} />
                <span className="sk sk-line" style={{ width: "82%" }} />
                <span className="sk sk-line" style={{ width: "66%" }} />
              </div>
            </div>
          </div>
        ))}
      </div>
      <div className="dashboard-skeleton-caption">
        <div className="dashboard-skeleton-title">正在安全地加载用量数据…</div>
        <div className="dashboard-skeleton-detail">首次加载期间保持页面结构稳定。</div>
      </div>
    </div>
  );
}

export function Dashboard({ report, loading, error, nowMs, onRefresh }: Props) {
  const content = useMemo(() => {
    if (report) {
      return (
        <>
          <OverviewCard report={report} />
          <section className="dashboard-details">
            <DetailsHeader count={report.windows.length} />
            <div className="quota-grid">
              {report.windows.map((window) => (
                <QuotaCard key={window.key} window={window} nowMs={nowMs} />
              ))}
            </div>
          </section>
        </>
      );
    }
    if (loading) {
      return <DashboardSkeleton />;
    }
    return (
      <Card className="dashboard-empty">
        <EmptyState
          icon={<Icons.Alert size={20} />}
          title="还没有配额数据"
          detail="点击下方按钮立即刷新，或等待自动检查。"
          action={
            <button type="button" className="btn btn-primary" onClick={onRefresh}>
              立即刷新
            </button>
          }
        />
      </Card>
    );
  }, [report, loading, nowMs, onRefresh]);

  return (
    <div className="dashboard">
      {content}
      {error ? (
        <Notice kind="warning">暂时无法更新配额数据：{error.user}</Notice>
      ) : null}
    </div>
  );
}
