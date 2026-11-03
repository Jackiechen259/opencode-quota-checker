// Separate Go usage page: monthly summary, cumulative/forecast chart, limits,
// model mix, and the Extra usage card.
//
// The page prefers the Console Go status/activity endpoints. If those are
// unavailable it still renders the summary and limits from the existing Go
// quota report, approximating the chart from the current percentage. Billing
// credit is optional and never blocks the page.

import { useMemo } from "react";
import { useGoOverview } from "../hooks/useApp";
import { compact, countdown, number, usdFromMicrocents } from "../lib/format";
import { buildGoSeries, formatGoDate, modelDisplayName, modelShare } from "../lib/goUsage";
import type {
  AppError,
  GoActivityReport,
  GoMeter,
  GoOverviewDto,
  GoStatusReport,
  TokenUsageDto,
  UsageReport,
  WindowReport,
} from "../types/models";
import { api } from "../services/tauri";
import { Card, EmptyState, Notice } from "../components/common";
import { GoUsageChart } from "../components/GoUsageChart";
import { Icons } from "../components/icons";

const DAY_MS = 86_400_000;

interface Props {
  report: UsageReport | null;
  loading: boolean;
  token: TokenUsageDto;
  nowMs: number;
  onRefresh: () => void;
}

interface LimitRow {
  key: string;
  label: string;
  percent: number;
  resetAt: number;
  limited: boolean;
  estimated: boolean;
}

function productLabel(product: string | undefined): string {
  switch (product) {
    case "go-plus":
      return "OpenCode Go Plus";
    case "go-teams":
      return "OpenCode Go Teams";
    default:
      return "OpenCode Go";
  }
}

function quotaWindow(report: UsageReport | null, key: string): WindowReport | undefined {
  return report?.windows.find((window) => window.key === key);
}

function meterRow(
  key: string,
  label: string,
  meter: GoMeter | null | undefined,
): LimitRow | null {
  if (!meter) return null;
  return {
    key,
    label,
    percent: meter.percent,
    resetAt: meter.resets_at,
    limited: meter.limited,
    estimated: false,
  };
}

function quotaRow(
  key: string,
  label: string,
  window: WindowReport | undefined,
): LimitRow | null {
  if (!window) return null;
  return {
    key,
    label,
    percent: window.percent,
    resetAt: window.reset_time,
    limited: window.limited ?? false,
    estimated: true,
  };
}

function limitRows(
  report: UsageReport | null,
  status: GoStatusReport | null,
): LimitRow[] {
  const access = status?.access ?? null;
  const rows = [
    meterRow("rolling-5h", "滚动 5 小时", access?.five_hour) ??
      quotaRow("rolling-5h", "滚动 5 小时", quotaWindow(report, "rolling-5h")),
    meterRow("weekly", "每周", access?.week) ??
      quotaRow("weekly", "每周", quotaWindow(report, "weekly")),
    meterRow("monthly", "每月", access?.month) ??
      quotaRow("monthly", "每月", quotaWindow(report, "monthly")),
  ];
  return rows.filter((row): row is LimitRow => row !== null);
}

function limitTone(row: LimitRow): string {
  if (row.limited || row.percent >= 90) return "is-critical";
  if (row.percent >= 70) return "is-warning";
  return "is-healthy";
}

function errorText(error: AppError | null): string {
  return error?.user ?? "未知错误";
}

function GoLimitsCard({ rows, nowMs }: { rows: LimitRow[]; nowMs: number }) {
  return (
    <Card className="go-card">
      <div className="go-card-heading">
        <h2>限额</h2>
        <p>各窗口的重置时间</p>
      </div>
      <div className="go-limits">
        {rows.length === 0 ? (
          <p className="go-muted">暂无配额窗口数据。</p>
        ) : (
          rows.map((row) => {
            const seconds = Math.max(0, Math.floor((row.resetAt - nowMs) / 1_000));
            return (
              <div className="go-limit" key={row.key}>
                <div className="go-limit-head">
                  <span className="go-limit-label">{row.label}</span>
                  <span className="go-limit-reset">
                    {seconds <= 0 ? "即将重置" : `${countdown(seconds)}后重置`}
                  </span>
                </div>
                <div className="go-limit-bar">
                  <i
                    className={limitTone(row)}
                    style={{ width: `${Math.min(100, Math.max(0, row.percent))}%` }}
                  />
                </div>
                <div className="go-limit-meta">
                  <span>{Math.round(row.percent)}% 已用</span>
                  {row.limited ? <span className="go-badge-danger">已达上限</span> : null}
                  {row.estimated ? <span className="go-muted">配额接口</span> : null}
                </div>
              </div>
            );
          })
        )}
      </div>
    </Card>
  );
}

function GoModelsCard({
  activity,
  token,
  activityError,
  onRefresh,
}: {
  activity: GoActivityReport | null;
  token: TokenUsageDto;
  activityError: AppError | null;
  onRefresh: () => void;
}) {
  const models = useMemo(() => {
    if (!activity) return [];
    const grouped = new Map<string, { model: string; provider: string; requests: number }>();
    for (const model of activity.models) {
      const current = grouped.get(model.model);
      if (current) {
        current.requests += model.requests;
      } else {
        grouped.set(model.model, { ...model });
      }
    }
    return [...grouped.values()].sort((a, b) => b.requests - a.requests);
  }, [activity]);

  const totalModelRequests = models.reduce((sum, model) => sum + model.requests, 0);
  const totalRequests = activity?.total_requests ?? token.report?.requests ?? null;
  const totalTokens = activity?.total_tokens ?? token.report?.total_tokens ?? null;
  const fallback = !activity && token.report !== null;

  return (
    <Card className="go-card">
      <div className="go-card-heading">
        <h2>模型</h2>
        <p>本期最常用</p>
      </div>
      {activityError ? (
        <Notice kind="warning">Go 模型明细暂不可用：{errorText(activityError)}</Notice>
      ) : null}
      {models.length === 0 ? (
        <p className="go-muted">
          {activityError ? "无法读取本期模型明细。" : "本期暂无 Go 请求。"}
        </p>
      ) : (
        <ul className="go-models">
          {models.slice(0, 3).map((model) => (
            <li className="go-model" key={model.model}>
              <span className="go-model-badge" aria-hidden="true">
                {modelDisplayName(model.model).slice(0, 2).toUpperCase()}
              </span>
              <span className="go-model-name">{modelDisplayName(model.model)}</span>
              <span className="go-model-share">
                {modelShare(model.requests, totalModelRequests)}
              </span>
            </li>
          ))}
        </ul>
      )}
      <div className="go-models-totals">
        <div>
          <span>Requests</span>
          <strong>{totalRequests === null ? "—" : number(totalRequests)}</strong>
        </div>
        <div>
          <span>Tokens</span>
          <strong>{totalTokens === null ? "—" : compact(totalTokens)}</strong>
        </div>
      </div>
      {fallback ? (
        <p className="go-muted">请求/Token 回退为近 30 天工作区汇总（全部模型）。</p>
      ) : null}
      {activityError ? (
        <button type="button" className="btn btn-secondary go-card-action" onClick={onRefresh}>
          重试
        </button>
      ) : null}
    </Card>
  );
}

function GoExtraUsageCard({
  status,
  billing,
  billingError,
}: {
  status: GoStatusReport | null;
  billing: GoOverviewDto["billing"];
  billingError: AppError | null;
}) {
  const useBalance = status?.use_balance ?? null;
  const stateText =
    useBalance === true
      ? "已开启 · 超出限额后使用余额"
      : useBalance === false
        ? "已关闭 · 达到限额后暂停请求"
        : "在 Console 中查看";
  const credit = billing ? usdFromMicrocents(billing.available_microcents) : "—";

  return (
    <Card className="go-card">
      <div className="go-card-heading">
        <h2>额外用量</h2>
        <p>{stateText}</p>
      </div>
      <div className="go-extra-credit">
        <span>可用额度</span>
        <strong>{credit}</strong>
      </div>
      {billingError ? (
        <p className="go-muted">余额暂不可用：{errorText(billingError)}</p>
      ) : null}
      <button
        type="button"
        className="btn btn-secondary go-card-action"
        onClick={() => void api.openGoPage().catch((error) => console.error("[go] open page failed", error))}
      >
        在 Console 管理
      </button>
    </Card>
  );
}

function GoSkeleton() {
  return (
    <div className="go-page" aria-busy="true" aria-label="正在加载 Go 用量">
      <Card>
        <span className="sk sk-line" style={{ width: 120 }} aria-hidden="true" />
        <span className="sk sk-line" style={{ width: 260, height: 44 }} aria-hidden="true" />
        <span className="sk sk-bar" aria-hidden="true" />
        <span className="sk sk-bar" aria-hidden="true" />
      </Card>
      <div className="go-grid" aria-hidden="true">
        {[0, 1, 2].map((index) => (
          <Card key={index}>
            <span className="sk sk-line" style={{ width: 90 }} />
            <span className="sk sk-line" style={{ width: "80%" }} />
            <span className="sk sk-line" style={{ width: "60%" }} />
          </Card>
        ))}
      </div>
    </div>
  );
}

export function GoUsagePage({ report, loading, token, nowMs, onRefresh }: Props) {
  const { data } = useGoOverview();
  const status = data.status;
  const activity = data.activity;
  const monthlyMeter = status?.access?.month ?? null;
  const monthlyWindow = quotaWindow(report, "monthly");
  const percent = monthlyMeter?.percent ?? monthlyWindow?.percent ?? 0;
  const end = monthlyMeter?.resets_at ?? monthlyWindow?.reset_time ?? nowMs;
  const explicitStart = status?.access?.starts_at ?? activity?.starts_at;
  const approximateStart = end - 30 * DAY_MS;
  const start =
    explicitStart !== undefined && explicitStart < end ? explicitStart : approximateStart;
  const limited = monthlyMeter?.limited || monthlyWindow?.limited || percent >= 100;
  const series = buildGoSeries(activity?.daily ?? [], percent, start, end, nowMs);
  const rows = useMemo(() => limitRows(report, status), [report, status]);
  const product = productLabel(status?.product);
  const reachesLimitBeforeRenewal =
    series.runsOut !== null && series.runsOut < end - DAY_MS;
  const overLimit =
    !limited && series.forecast !== null && series.forecast > 100;
  const forecastLabel = limited
    ? "本月额度已用尽"
    : reachesLimitBeforeRenewal && series.runsOut !== null
      ? `预计 ${formatGoDate(series.runsOut)} 达到上限`
      : series.forecast !== null
        ? `预计 ${Math.round(series.forecast)}%`
        : "数据积累中";
  const hasAnyData = report !== null || status !== null || activity !== null;
  const chartLoading = data.loading && activity === null;

  if (!hasAnyData) {
    if (loading || data.loading) return <GoSkeleton />;
    return (
      <div className="go-page">
        <Card className="dashboard-empty">
          <EmptyState
            icon={<Icons.Activity size={20} />}
            title="还没有 Go 用量数据"
            detail={
              data.statusError?.user ??
              "点击下方按钮刷新，或等待后台检查。"
            }
            action={
              <button type="button" className="btn btn-primary" onClick={onRefresh}>
                立即刷新
              </button>
            }
          />
        </Card>
      </div>
    );
  }

  return (
    <div className="go-page">
      <div className="go-page-head">
        <div>
          <span className="go-product">{product}</span>
          <h1>Go 用量</h1>
        </div>
        <button
          type="button"
          className="btn btn-secondary"
          onClick={() => void api.openGoPage().catch((error) => console.error("[go] open page failed", error))}
        >
          在 Console 打开
        </button>
      </div>

      {data.statusError ? (
        <Notice kind="warning">
          {status
            ? "Go 状态更新失败，显示上次数据："
            : "订阅状态暂不可用，已按配额接口显示："}
          {errorText(data.statusError)}
        </Notice>
      ) : null}

      <Card className="go-summary-card">
        <div className="go-summary">
          <div className="go-summary-main">
            <span className={`go-percent ${limited ? "is-critical" : ""}`}>
              {Math.round(percent)}%
            </span>
            <div className="go-summary-copy">
              <span className="go-summary-title">月度已用</span>
              <span className={`go-summary-sub ${overLimit ? "is-warning" : ""}`}>
                {formatGoDate(end)} 续订 · {forecastLabel}
              </span>
            </div>
          </div>
        </div>
        {chartLoading ? (
          <div className="go-chart-skeleton" aria-label="正在加载 Go 明细" aria-busy="true">
            <span className="sk sk-line" style={{ width: "100%", height: 200 }} />
          </div>
        ) : (
          <GoUsageChart
            series={series}
            startsAt={start}
            endsAt={end}
            percent={percent}
          />
        )}
        {data.activityError && !activity ? (
          <p className="go-muted">Go 明细暂不可用，曲线按当前进度估算。</p>
        ) : null}
      </Card>

      <div className="go-grid">
        <GoLimitsCard rows={rows} nowMs={nowMs} />
        {chartLoading ? (
          <Card className="go-card" aria-busy="true" aria-label="正在加载模型明细">
            <span className="sk sk-line" style={{ width: 100 }} />
            <span className="sk sk-line" style={{ width: "80%" }} />
            <span className="sk sk-line" style={{ width: "65%" }} />
          </Card>
        ) : (
          <GoModelsCard
            activity={activity}
            token={token}
            activityError={data.activityError}
            onRefresh={onRefresh}
          />
        )}
        <GoExtraUsageCard
          status={status}
          billing={data.billing}
          billingError={data.billingError}
        />
      </div>
    </div>
  );
}
