// Workspace token usage card: totals, token buckets, requests, and the
// Console-estimated cost for the last 30 days.
//
// The data is best-effort: a token-usage failure never affects the quota
// cards. The card therefore owns its own loading/error presentation and keeps
// the last successful report visible when a refresh fails.

import { api } from "../services/tauri";
import { compact, number, relative, usdFromMicrocents } from "../lib/format";
import type { AppError, TokenUsageDto, TokenUsageReport } from "../types/models";
import { Card, Notice, SectionHeader } from "./common";
import { Icons } from "./icons";

const RANGE_LABELS: Record<string, string> = {
  "24h": "近 24 小时",
  "7d": "近 7 天",
  "30d": "近 30 天",
  "6m": "近 6 个月",
  all: "全部时间",
};

function rangeLabel(range: string): string {
  return RANGE_LABELS[range] ?? `近 ${range}`;
}

function permissionHint(error: AppError): boolean {
  return (
    error.code === "usage_permission_denied" ||
    error.code === "authentication_failed"
  );
}

function openConsole() {
  void api.openConsolePage().catch((caught) => {
    console.error("[token-usage] open_console_page failed", caught);
  });
}

function tokenErrorMessage(error: AppError): string {
  if (error.code === "parse_failed") {
    return "Token 用量接口返回了无法识别的数据。";
  }
  return error.user;
}

function TokenStat({
  label,
  value,
  exact,
}: {
  label: string;
  value: string;
  exact: string;
}) {
  return (
    <div className="stat-tile">
      <span className="stat-tile-label">{label}</span>
      <span className="stat-tile-value" title={exact}>
        {value}
      </span>
    </div>
  );
}

function TokenError({ error }: { error: AppError }) {
  return (
    <div className="token-usage-error">
      <Notice kind="warning">暂时无法读取 Token 用量：{tokenErrorMessage(error)}</Notice>
      {permissionHint(error) ? (
        <button type="button" className="btn btn-secondary" onClick={openConsole}>
          打开 Console
        </button>
      ) : null}
    </div>
  );
}

function TokenSkeleton() {
  return (
    <Card>
      <div
        className="token-usage-skeleton"
        aria-busy="true"
        aria-label="正在加载 Token 用量"
      >
        <span className="sk sk-line" style={{ width: 180, height: 32 }} aria-hidden="true" />
        <span className="sk sk-line" style={{ width: 120 }} aria-hidden="true" />
        <div className="overview-stats token-usage-stats" aria-hidden="true">
          {[0, 1, 2, 3].map((index) => (
            <span className="sk sk-tile" key={index} />
          ))}
        </div>
      </div>
    </Card>
  );
}

function TokenData({
  report,
  token,
  nowMs,
}: {
  report: TokenUsageReport;
  token: TokenUsageDto;
  nowMs: number;
}) {
  const updatedAt = token.lastSuccessMs ?? report.fetched_at;
  return (
    <Card className="token-usage-card">
      <div className="overview-hero token-usage-hero">
        <div className="token-usage-total">
          <span
            className="overview-hero-value"
            title={`${number(report.total_tokens)} 个 Token`}
          >
            {compact(report.total_tokens)}
          </span>
          <span className="token-usage-total-label">总 Token</span>
        </div>
        <span className="token-usage-cost" title="按模型牌价折算，非实际扣费">
          折算费用 {usdFromMicrocents(report.cost_microcents)}
        </span>
      </div>
      <div className="overview-stats token-usage-stats">
        <TokenStat
          label="输入"
          value={compact(report.input_tokens)}
          exact={number(report.input_tokens)}
        />
        <TokenStat
          label="输出"
          value={compact(report.output_tokens)}
          exact={number(report.output_tokens)}
        />
        <TokenStat
          label="缓存读取"
          value={compact(report.cache_read_tokens)}
          exact={number(report.cache_read_tokens)}
        />
        <TokenStat
          label="缓存写入"
          value={compact(report.cache_write_tokens)}
          exact={number(report.cache_write_tokens)}
        />
      </div>
      <div className="token-usage-meta">
        <span>{token.loading ? "更新中…" : `更新于 ${relative(updatedAt, nowMs)}`}</span>
        <span>请求 {number(report.requests)}</span>
        <span className="token-usage-note">
          含缓存读取；费用按模型牌价折算，非实际扣费
        </span>
      </div>
      {token.error ? (
        <div className="token-usage-error">
          <Notice kind="warning">
            Token 用量更新失败：{tokenErrorMessage(token.error)}
          </Notice>
          {permissionHint(token.error) ? (
            <button type="button" className="btn btn-secondary" onClick={openConsole}>
              打开 Console
            </button>
          ) : null}
        </div>
      ) : null}
    </Card>
  );
}

export function TokenUsageCard({
  token,
  nowMs,
}: {
  token: TokenUsageDto;
  nowMs: number;
}) {
  if (!token.report && token.error) {
    return (
      <section className="dashboard-details token-usage">
        <SectionHeader
          icon={<Icons.Activity size={16} />}
          title="Token 用量"
          subtitle="当前工作区 · 全部模型 · 近 30 天"
        />
        <Card>
          <TokenError error={token.error} />
        </Card>
      </section>
    );
  }

  if (!token.report) {
    return (
      <section className="dashboard-details token-usage">
        <SectionHeader
          icon={<Icons.Activity size={16} />}
          title="Token 用量"
          subtitle="当前工作区 · 全部模型 · 近 30 天"
          trailing={<span className="details-count">正在加载…</span>}
        />
        <TokenSkeleton />
      </section>
    );
  }

  return (
    <section className="dashboard-details token-usage">
      <SectionHeader
        icon={<Icons.Activity size={16} />}
        title="Token 用量"
        subtitle={`当前工作区 · 全部模型 · ${rangeLabel(token.report.range)}`}
      />
      <TokenData report={token.report} token={token} nowMs={nowMs} />
    </section>
  );
}
