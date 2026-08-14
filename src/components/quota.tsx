// Quota visualization components: progress ring, quota card, overview hero.

import { countdown, number, percent } from "../lib/format";
import type { UsageReport, WindowReport } from "../types/models";
import { quotaHealth } from "../types/models";
import { Card, Divider, Dot, Metric, ProgressBar, SectionDivider, SectionHeader, StatusBadge, healthColor, progressColor } from "./common";
import { Icons } from "./icons";

/** SVG ring showing the used percentage. */
export function QuotaRing({ percent, health, size = 112 }: { percent: number; health: string; size?: number }) {
  const stroke = 10;
  const radius = (size - stroke) / 2;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.min(100, Math.max(0, percent));
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
        fontSize="32"
        fontWeight="600"
        fill="var(--color-text-primary)"
      >
        {clamped.toFixed(0)}
        <tspan fontSize="14" fill="var(--color-text-muted)">
          %
        </tspan>
      </text>
    </svg>
  );
}

function healthTone(health: "healthy" | "warning" | "critical"): "success" | "warning" | "danger" {
  return health === "critical" ? "danger" : health === "warning" ? "warning" : "success";
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
          {health === "healthy" ? "正常" : health === "warning" ? "警告" : "危险"}
        </span>
        <span className="quota-card-title">{window.label}</span>
        <StatusBadge tone={healthTone(health)}>{health === "healthy" ? "健康" : health === "warning" ? "接近阈值" : "危险"}</StatusBadge>
      </div>
      <div className="quota-card-body">
        <QuotaRing percent={window.percent} health={health} />
        <div className="quota-card-metrics">
          <Metric label="已用" value={number(window.used)} />
          <Metric label="总额" value={number(window.quota)} />
          <Metric label="剩余" value={number(window.remaining)} />
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
  const barColor = progressColor(health);
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
        <StatusBadge>{highest.label}</StatusBadge>
        <span style={{ flex: 1 }} />
      </div>
      <div className="overview-hero">
        <span className="overview-hero-value" style={{ color: accent }}>
          {percent(highest.percent)}
        </span>
        <span style={{ flex: 1 }} />
        <span className="overview-hero-countdown">
          <Icons.Clock size={16} className="overview-hero-clock" />
          {countdown(highest.reset_in_secs)}
        </span>
      </div>
      <ProgressBar percent={highest.percent} color={barColor} />
      <SectionDivider />
      <div className="overview-stats">
        <Metric label="已用" value={number(highest.used)} />
        <Divider height={40} />
        <Metric label="总额" value={number(highest.quota)} />
        <Divider height={40} />
        <Metric label="剩余" value={number(highest.remaining)} />
        <Divider height={40} />
        <div className="overview-health">
          <Metric label="窗口健康" value={`${counts.healthy} / ${total}`} />
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
