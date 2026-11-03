// Display formatting ported from the archived Iced `view/format.rs`.

/** Formats a Unix millisecond timestamp for display. */
export function timestamp(timestampMs: number): string {
  const date = new Date(timestampMs);
  if (Number.isNaN(date.getTime())) return "未知时间";
  const pad = (value: number) => String(value).padStart(2, "0");
  return (
    `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ` +
    `${pad(date.getUTCHours())}:${pad(date.getUTCMinutes())}:${pad(date.getUTCSeconds())} UTC`
  );
}

/** Formats a signed number of seconds as a reset countdown. */
export function countdown(seconds: number): string {
  if (seconds <= 0) return "即将重置";
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  if (days > 0) return `${days} 天 ${hours} 小时`;
  if (hours > 0) return `${hours} 小时 ${minutes} 分`;
  return `${minutes} 分 ${seconds % 60} 秒`;
}

/** Short countdown for compact displays (e.g. `04:54` or `1天`). */
export function countdownShort(seconds: number): string {
  if (seconds <= 0) return "即将重置";
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  const secs = seconds % 60;
  if (days > 0) return `${days}天 ${hours}时`;
  if (hours > 0) return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
  return `${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

/** Human-friendly "time ago" relative to `nowMs`. */
export function relative(fetchedAtMs: number, nowMs: number): string {
  const delta = Math.max(0, Math.floor((nowMs - fetchedAtMs) / 1_000));
  if (delta < 5) return "刚刚";
  if (delta < 60) return `${delta} 秒前`;
  const minutes = Math.floor(delta / 60);
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  return `${Math.floor(hours / 24)} 天前`;
}

/**
 * Formats a number with thousands separators and at most one decimal.
 *
 * A whole value is printed without a redundant trailing zero ("100", not
 * "100.0"); a real fraction keeps its single decimal ("75,780.9").
 */
export function number(value: number): string {
  return value.toLocaleString("en-US", {
    maximumFractionDigits: 1,
  });
}

/**
 * Formats a percentage as a whole number, tolerating values over 100.
 *
 * Percentages are rounded for display ("78%", "52%") so a quota readout does
 * not carry a meaningless decimal; the underlying value stays untouched and
 * every bar, ring and threshold decision keeps using the exact number.
 */
export function percent(value: number): string {
  return `${Math.round(value)}%`;
}

/** Relative "last checked" label from an epoch-milliseconds timestamp. */
export function lastCheckedText(timestampMs: number | null): string {
  if (timestampMs === null) return "从未";
  const seconds = Math.max(0, Math.floor((Date.now() - timestampMs) / 1_000));
  if (seconds <= 59) return "刚刚";
  if (seconds < 3_600) return `${Math.floor(seconds / 60)} 分钟前`;
  if (seconds < 86_400) return `${Math.floor(seconds / 3_600)} 小时前`;
  return `${Math.floor(seconds / 86_400)} 天前`;
}

/** Bytes → human readable size, without a redundant trailing `.0`. */
export function bytes(value: number): string {
  if (value < 1024) return `${value} B`;
  const units = ["KB", "MB", "GB"];
  let amount = value;
  let unit = "B";
  for (const next of units) {
    if (amount < 1024) break;
    amount /= 1024;
    unit = next;
  }
  return `${amount.toLocaleString("en-US", { maximumFractionDigits: 1 })} ${unit}`;
}

/**
 * Compact token count for cards, using Chinese 万/亿 units.
 *
 * Values below ten thousand keep the exact formatter so a small count is not
 * rounded into a misleading unit ("8,534", not "8534"). Larger values use the
 * zh-CN compact notation ("1万", "2601.78万", "12.6亿"). Non-finite input is a
 * bug, not a number to display.
 */
export function compact(value: number): string {
  if (!Number.isFinite(value)) return "—";
  if (Math.abs(value) < 10_000) return number(value);
  return value.toLocaleString("zh-CN", {
    notation: "compact",
    maximumFractionDigits: 2,
  });
}

/**
 * Converts Console microcents to a display-only USD amount.
 *
 * 1 USD = 100,000,000 microcents. The division happens only at display time;
 * the report keeps the exact integer.
 */
export function usdFromMicrocents(value: number): string {
  if (!Number.isFinite(value)) return "—";
  const dollars = value / 100_000_000;
  return `$${dollars.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`;
}
