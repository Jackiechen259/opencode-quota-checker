// Pure helpers for the Go usage page: cumulative series, forecast, date labels,
// and model display names. Kept out of the React component so the chart math is
// unit-testable and the component only renders.

import type { GoActivityDay } from "../types/models";

const DAY_MS = 86_400_000;

/** One point on the cumulative usage line. */
export interface GoSeriesPoint {
  /** Normalized x position, 0 = period start, 1 = period end. */
  x: number;
  /** Cumulative share of the monthly allowance, 0-100. */
  value: number;
  /** UTC date for daily points (`YYYY-MM-DD`). */
  date?: string;
  /** Requests on that day. */
  requests?: number;
  /** Percent added by that day. */
  dailyValue?: number;
}

export interface GoSeries {
  points: GoSeriesPoint[];
  /** Linear extrapolation to the period end, or null when not meaningful. */
  forecast: number | null;
  /** Epoch millis when the forecast crosses 100%, if it does. */
  runsOut: number | null;
  /** Average percent per day so far. */
  perDay: number | null;
  /** Sum of the daily cost values. */
  totalCost: number;
  /** No usage yet. */
  empty: boolean;
  /** True when the line is a linear approximation because daily data is absent. */
  estimated: boolean;
}

/**
 * Builds the cumulative usage series for the chart.
 *
 * The Console reports the monthly percentage and a Go-only daily cost series.
 * Each day's cumulative percentage is the monthly percentage scaled by that
 * day's share of the period cost; when the daily series is missing or empty the
 * line falls back to a straight start-to-now estimate and `estimated` is true.
 */
export function buildGoSeries(
  daily: GoActivityDay[],
  percent: number,
  startsAt: number,
  endsAt: number,
  now: number,
): GoSeries {
  const period = endsAt - startsAt;
  if (!Number.isFinite(period) || period <= 0) {
    return {
      points: [],
      forecast: null,
      runsOut: null,
      perDay: null,
      totalCost: 0,
      empty: percent <= 0,
      estimated: true,
    };
  }

  const elapsed = Math.min(Math.max(now - startsAt, 0), period);
  const limited = percent >= 100;
  const expired = now >= endsAt;
  const forecast =
    !expired && !limited && elapsed >= DAY_MS && percent > 0
      ? (percent * period) / elapsed
      : null;
  const perDay =
    !expired && elapsed >= DAY_MS && percent > 0
      ? (percent * DAY_MS) / elapsed
      : null;
  const runsOut =
    forecast !== null && forecast > 100 && percent > 0
      ? startsAt + (elapsed * 100) / percent
      : null;

  const ordered = [...daily].sort((a, b) => a.date.localeCompare(b.date));
  const totalCost = ordered.reduce((sum, day) => sum + day.cost_microcents, 0);
  const points: GoSeriesPoint[] = [{ x: 0, value: 0 }];
  let estimated = true;

  if (ordered.length > 0 && totalCost > 0) {
    estimated = false;
    let cumulativeCost = 0;
    for (const day of ordered) {
      const dayStart = Date.parse(`${day.date}T00:00:00Z`);
      if (!Number.isFinite(dayStart)) continue;
      const dayEnd = Math.min(now, dayStart + DAY_MS);
      const x = Math.min(Math.max((dayEnd - startsAt) / period, 0), 1);
      const dailyValue = (percent * day.cost_microcents) / totalCost;
      cumulativeCost += day.cost_microcents;
      points.push({
        x,
        value: (percent * cumulativeCost) / totalCost,
        date: day.date,
        requests: day.requests,
        dailyValue,
      });
    }
    // The daily rollup may lag the current percentage; extend the line to now
    // so the chart always ends at the reported value.
    const currentX = Math.min(Math.max(elapsed / period, 0), 1);
    const last = points[points.length - 1];
    if (last.x < currentX) {
      points.push({ x: currentX, value: percent });
    }
  } else {
    points.push({
      x: Math.min(Math.max(elapsed / period, 0), 1),
      value: percent,
    });
  }

  return {
    points,
    forecast,
    runsOut,
    perDay,
    totalCost,
    empty: percent <= 0,
    estimated,
  };
}

/** Formats an epoch timestamp as `11 月 26 日` in UTC. */
export function formatGoDate(ms: number): string {
  const date = new Date(ms);
  return `${date.getUTCMonth() + 1} 月 ${date.getUTCDate()} 日`;
}

/** Formats a `YYYY-MM-DD` date for the chart axis, e.g. `10/26`. */
export function formatAxisDate(date: string): string {
  const [, month, day] = date.split("-");
  if (!month || !day) return date;
  return `${month}/${day}`;
}

const MODEL_NAMES: Record<string, string> = {
  "deepseek-v4.1-flash": "DeepSeek V4.1 Flash",
  "deepseek-v4-pro": "DeepSeek V4 Pro",
  "glm-5.3": "GLM-5.3",
  "glm-5.3-flash": "GLM-5.3 Flash",
  "glm-5.2": "GLM-5.2",
  "kimi-k3.1-pro": "Kimi K3.1 Pro",
  "kimi-k3": "Kimi K3",
  "kimi-k2.7-code": "Kimi K2.7 Code",
  "gpt-6-luna": "GPT-6 Luna",
  "gpt-5.6-luna": "GPT-5.6 Luna",
  "claude-haiku-5-5": "Claude Haiku 5.5",
  "gemini-4-argon": "Gemini 4 Argon",
  "grok-4.7": "Grok 4.7",
  "qwen3.8-max": "Qwen3.8 Max",
  "qwen3.8-flash": "Qwen3.8 Flash",
  "qwen3.7-plus": "Qwen3.7 Plus",
  "minimax-m3.1-flash": "MiniMax M3.1 Flash",
  "minimax-m3": "MiniMax M3",
  "mimo-v2.6-flash": "MiMo V2.6 Flash",
  "mimo-v2.6-pro": "MiMo V2.6 Pro",
  "mimo-v2.5": "MiMo V2.5",
  "mimo-v2.5-pro": "MiMo V2.5 Pro",
  "longcat-2.0": "LongCat 2.0",
  "step-5-preview": "Step 5 Preview",
  "hy4-preview": "HY4 Preview",
  hy3: "HY3",
  "muse-spark-1.3-contributor": "Muse Spark 1.3 Contributor",
  "muse-spark-1.2-contributor": "Muse Spark 1.2 Contributor",
  "beam-free": "Beam Free",
  "iceman-contributor-free": "Iceman Contributor Free",
};

/** Human-readable model name, falling back to a title-cased identifier. */
export function modelDisplayName(model: string): string {
  const known = MODEL_NAMES[model];
  if (known) return known;
  return model
    .split("-")
    .map((part) => (part ? part[0].toUpperCase() + part.slice(1) : part))
    .join(" ");
}

/** Request share label matching the Console: `99%`, `<1%`, `0%`. */
export function modelShare(requests: number, total: number): string {
  if (total <= 0 || requests <= 0) return "0%";
  const share = (requests / total) * 100;
  if (share > 0 && share < 1) return "<1%";
  return `${Math.round(share)}%`;
}
