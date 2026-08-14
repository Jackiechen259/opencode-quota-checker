// Dashboard page: overview hero + quota window grid.

import { useMemo } from "react";
import { DetailsHeader, OverviewCard, QuotaCard } from "../components/quota";
import { Card, EmptyState, Notice, Spinner } from "../components/common";
import type { UsageReport } from "../types/models";
import { Icons } from "../components/icons";

interface Props {
  report: UsageReport | null;
  loading: boolean;
  error: { user: string } | null;
  nowMs: number;
  onRefresh: () => void;
}

export function Dashboard({ report, loading, error, nowMs, onRefresh }: Props) {
  const content = useMemo(() => {
    if (report) {
      const columns = window.innerWidth > 1180 ? 3 : window.innerWidth >= 820 ? 2 : 1;
      const rows: (typeof report.windows)[number][][] = [];
      for (let index = 0; index < report.windows.length; index += columns) {
        rows.push(report.windows.slice(index, index + columns));
      }
      return (
        <>
          <OverviewCard report={report} />
          <section className="dashboard-details">
            <DetailsHeader count={report.windows.length} />
            <div
              className="quota-grid"
              style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
            >
              {report.windows.map((window) => (
                <QuotaCard key={window.key} window={window} nowMs={nowMs} />
              ))}
            </div>
          </section>
        </>
      );
    }
    if (loading) {
      return (
        <Card className="dashboard-skeleton">
          <Spinner size={48} />
          <div className="dashboard-skeleton-title">正在安全地加载用量数据…</div>
          <div className="dashboard-skeleton-detail">首次加载期间保持页面结构稳定。</div>
        </Card>
      );
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
