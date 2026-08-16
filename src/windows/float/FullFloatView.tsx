// Full floating view: meta row + scrollable quota list + footer.
//
// Header and meta stay fixed; only the quota list scrolls when the window
// reaches FULL_MAX_HEIGHT. The footer's update time is the only ticking
// piece, isolated in its own component.

import { Dot } from "../../components/common";
import { quotaHealth, type UsageReport } from "../../types/models";
import { highestWindow, healthColors } from "./floatLayout";
import { FloatStateView } from "./FloatStateView";
import { QuotaCard } from "./QuotaCard";
import { RelativeUpdateTime } from "./tickers";

export function FullFloatView({
  report,
  loading,
  error,
  onRetry,
}: {
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  onRetry: () => void;
}) {
  if (!report || report.windows.length === 0) {
    return (
      <div className="float-full">
        <FloatStateView loading={loading} error={error} onRetry={onRetry} />
      </div>
    );
  }

  const highest = highestWindow(report);
  const highestAccent = healthColors[quotaHealth(highest?.percent ?? 0)];
  const plan = report.plan_type.trim() ? `OpenCode Go · ${report.plan_type}` : "OpenCode Go";

  return (
    <div className="float-full">
      <div className="float-meta">
        <span
          className="float-plan-badge"
          title={report.windows.length > 0 ? `共 ${report.windows.length} 个配额周期` : undefined}
        >
          {plan}
        </span>
        <span className="float-spacer" />
        <span className="float-meta-highest" style={{ color: highestAccent }}>
          最高 {highest ? highest.percent.toFixed(1) : "0.0"}%
        </span>
      </div>
      <div className="float-quota-list">
        {report.windows.map((window) => (
          <QuotaCard key={window.key} window={window} />
        ))}
      </div>
      <div className="float-footer-row">
        <span className="float-footer-monitor">
          <Dot color="var(--success)" size={6} />
          自动监测
        </span>
        <span className="float-spacer" />
        <RelativeUpdateTime fetchedMs={report.fetched_at} className="float-footer-time" />
      </div>
    </div>
  );
}
