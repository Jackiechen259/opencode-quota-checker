// Quota visualization components: progress ring, quota card, overview hero.

import { countdown, number, percent } from "../lib/format";
import type { UsageReport, WindowReport } from "../types/models";
import { quotaHealth } from "../types/models";
import {
  Card,
  Dot,
  ProgressBar,
  SectionDivider,
  SectionHeader,
  StatusBadge,
  healthColor,
} from "./common";
import { Icons } from "./icons";

/** SVG ring showing the used percentage. */
export function QuotaRing({
  percent: value,
  health,
  size = 104,
}: {
  percent: number;
  health: string;
  size?: number;
}) {
  const stroke = 8;
  const radius = (size - stroke) / 2;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.min(100, Math.max(0, value));
  const dash = (clamped / 100) * circumference;
  const color = healthColor(health as "healthy" | "warning" | "critical");
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="quota-ring">
      <circle cx={size / 2} cy={size / 2} r={radius} fill="none" stroke="var(--color-track)" strokeWidth={stroke} />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke={color}
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={`${dash} ${circumference}`}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
        style={{ transition: "stroke-dasharray 400ms ease" }}
      />
      <text
        x="50%"
        y="50%"
        textAnchor="middle"
        dominantBaseline="central"
        fontSize="30"
        fontWeight="650"
        fill="var(--color-text-primary)"
      >
        {clamped.toFixed(0)}
        <tspan fontSize="12" fill="var(--color-text-muted)">
          %
        </tspan>
      </text>
    </svg>
  );
}

function healthTone(health: "healthy" | "warning" | "critical"): "success" | "warning" | "danger" {
  return health === "critical" ? "danger" : health === "warning" ? "warning" : "success";
}

function healthText(health: "healthy" | "warning" | "critical"): string {
  return health === "healthy" ? "健康" : health === "warning" ? "接近阈值" : "危险";
}

function statusText(health: "healthy" | "warning" | "critical"): string {
  return health === "healthy" ? "正常" : health === "warning" ? "警告" : "危险";
}

/** One quota window card (ring + metrics + reset countdown). */
export function QuotaCard({ window, nowMs }: { window: WindowReport; nowMs: number }) {
  const health = quotaHealth(window.percent);
  const resetSeconds = Math.max(0, Math.floor((window.reset_time - nowMs) / 1_000));
  return (
    <Card className="quota-card">
      <div className="quota-card-header">
        <span className="quota-card-status">
          <Dot color={healthColor(health)} />
          {statusText(health)}
        </span>
        <span className="quota-card-title" title={window.label}>
          {window.label}
        </span>
        <StatusBadge tone={healthTone(health)}>{healthText(health)}</StatusBadge>
      </div>
      <div className="quota-card-body">
        <QuotaRing percent={window.percent} health={health} />
        <div className="quota-card-metrics">
          <div className="quota-card-metric-row">
            <span className="quota-card-metric-label">已用</span>
            <span className="quota-card-metric-value">{number(window.used)}</span>
          </div>
          <div className="quota-card-metric-row">
            <span className="quota-card-metric-label">剩余</span>
            <span className="quota-card-metric-value">{number(window.remaining)}</span>
          </div>
          <div className="quota-card-metric-row">
            <span className="quota-card-metric-label">总额</span>
            <span className="quota-card-metric-value">{number(window.quota)}</span>
          </div>
        </div>
      </div>
      <SectionDivider />
      <div className="quota-card-reset">
        <span className="quota-card-reset-label">下次重置</span>
        <span className="quota-card-reset-value">{countdown(resetSeconds)}</span>
      </div>
    </Card>
  );
}

/** Hero overview card for the highest-loaded window. */
export function OverviewCard({ report }: { report: UsageReport }) {
  const highest = report.windows.reduce<(typeof report.windows)[number] | null>(
    (best, window) => (!best || window.percent > best.percent ? window : best),
    null,
  );
  if (!highest) {
    return (
      <Card>
        <div className="overview-empty">暂无配额数据</div>
      </Card>
    );
  }
  const health = quotaHealth(highest.percent);
  const accent = healthColor(health);
  const barColor = healthColor(health);
  const counts = report.windows.reduce(
    (acc, window) => {
      const h = quotaHealth(window.percent);
      acc[h] += 1;
      return acc;
    },
    { healthy: 0, warning: 0, critical: 0 } as Record<"healthy" | "warning" | "critical", number>,
  );
  const total = report.windows.length;

  return (
    <Card className="overview-card">
      <div className="overview-heading">
        <span className="overview-heading-dot">
          <Dot color={accent} />
        </span>
        <span className="overview-heading-title">最高负载</span>
        <span style={{ flex: 1 }} />
        <StatusBadge>{highest.label}</StatusBadge>
      </div>
      <div className="overview-hero">
        <span className="overview-hero-value" style={{ color: accent }}>
          {percent(highest.percent)}
        </span>
        <span className="overview-hero-countdown">
          <Icons.Clock size={15} className="overview-hero-clock" />
          {countdown(highest.reset_in_secs)}
        </span>
      </div>
      <ProgressBar percent={highest.percent} color={barColor} />
      <div className="overview-stats">
        <div className="stat-tile">
          <span className="stat-tile-label">已使用</span>
          <span className="stat-tile-value">{number(highest.used)}</span>
        </div>
        <div className="stat-tile">
          <span className="stat-tile-label">剩余</span>
          <span className="stat-tile-value">{number(highest.remaining)}</span>
        </div>
        <div className="stat-tile">
          <span className="stat-tile-label">总额</span>
          <span className="stat-tile-value">{number(highest.quota)}</span>
        </div>
        <div className="stat-tile">
          <span className="stat-tile-label">窗口健康</span>
          <div className="overview-health">
            <span className="stat-tile-value">{counts.healthy} / {total}</span>
            <div className="overview-health-distribution">
              {([["healthy", "var(--color-success)"], ["warning", "var(--color-warning)"], ["critical", "var(--color-danger)"]] as const).map(
                ([key, color]) =>
                  counts[key] > 0 ? (
                    <span
                      key={key}
                      className="overview-health-segment"
                      style={{
                        background: color,
                        flexGrow: counts[key],
                      }}
                    />
                  ) : null,
              )}
            </div>
          </div>
        </div>
      </div>
    </Card>
  );
}

/** "详细指标" section header with window count. */
export function DetailsHeader({ count }: { count: number }) {
  return (
    <SectionHeader
      icon={<Icons.Layers size={16} />}
      title="详细指标"
      subtitle="各配额窗口明细"
      trailing={<span className="details-count">共 {count} 个窗口</span>}
    />
  );
}
