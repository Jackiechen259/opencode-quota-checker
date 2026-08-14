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

/** Formats a number with thousands separators and one decimal place. */
export function number(value: number): string {
  return value.toLocaleString("en-US", {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  });
}

/** Formats a percentage with one decimal, tolerating values over 100. */
export function percent(value: number): string {
  return `${value.toFixed(1)}%`;
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

/** Bytes → human readable size. */
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
  return `${amount.toFixed(1)} ${unit}`;
}
