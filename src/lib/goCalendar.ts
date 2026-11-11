// Pure math for the GitHub-style Go usage calendar.
//
// The grid renders the last 53 weeks (one column per week, Monday first, one
// cell per UTC day). Colors are ranked by the day's token count, so the busiest
// day in the window always reaches the deepest bucket even when the absolute
// numbers are uneven. Kept out of the component so the geometry and the
// statistics are unit-testable.

import type { GoActivityDay } from "../types/models";

const DAY_MS = 86_400_000;

/** Week columns rendered: a year plus the current partial week (like GitHub). */
export const CALENDAR_WEEKS = 53;

const MONTH_LABELS = [
  "1月",
  "2月",
  "3月",
  "4月",
  "5月",
  "6月",
  "7月",
  "8月",
  "9月",
  "10月",
  "11月",
  "12月",
];

/** One rendered day cell. */
export interface CalendarCell {
  /** UTC date, `YYYY-MM-DD`. */
  date: string;
  /** Tokens used on that day (0 when the day was not recorded). */
  tokens: number;
  /** Requests on that day. */
  requests: number;
  /** Cost in microcents (`1 USD = 100,000,000`). */
  costMicrocents: number;
  /** Usage bucket: 0 = no usage, 4 = the busiest bucket in the window. */
  level: number;
  /** After today: rendered as a placeholder and excluded from the totals. */
  future: boolean;
  /** The day has a recorded history row (a recorded zero is still a record). */
  recorded: boolean;
}

/** Month caption above a week column. */
export interface CalendarMonthLabel {
  /** Zero-based week column. */
  column: number;
  label: string;
}

/** Headline numbers shown under the grid. */
export interface GoCalendarStats {
  totalTokens: number;
  totalRequests: number;
  totalCostMicrocents: number;
  /** Days with a token count above zero. */
  activeDays: number;
  /** Consecutive active days ending today (or yesterday, while today is empty). */
  currentStreak: number;
  /** Longest run of consecutive active days in the window. */
  longestStreak: number;
  /** Busiest day, or null when the window has no usage. */
  best: CalendarCell | null;
  /** Earliest recorded day overall (may predate the visible window). */
  recordStart: string | null;
  /** First visible day, `YYYY-MM-DD`. */
  windowStart: string;
  /** Today in UTC, `YYYY-MM-DD`. */
  today: string;
}

export interface GoCalendar {
  /** 53 columns of 7 days, Monday first. */
  weeks: CalendarCell[][];
  months: CalendarMonthLabel[];
  stats: GoCalendarStats;
  /** No recorded usage in the visible window. */
  empty: boolean;
}

/** `YYYY-MM-DD` for an epoch-milliseconds instant, in UTC. */
export function utcDateKey(ms: number): string {
  return new Date(ms).toISOString().slice(0, 10);
}

function dayIndex(date: string): number {
  return Math.round(Date.parse(`${date}T00:00:00Z`) / DAY_MS);
}

function dateFromIndex(index: number): string {
  return new Date(index * DAY_MS).toISOString().slice(0, 10);
}

/** Whether a `YYYY-MM-DD` string is a real calendar date. */
function isValidDate(date: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return false;
  return dateFromIndex(dayIndex(date)) === date;
}

function tokenCount(day: GoActivityDay): number {
  return Number.isFinite(day.tokens) && day.tokens > 0 ? day.tokens : 0;
}

/**
 * Builds the year grid and its statistics.
 *
 * `nowMs` is the clock of the caller (the Go page passes its ticker), so the
 * grid shifts at UTC midnight and the tests can pin a date. Days the history
 * does not contain render as level 0 instead of being treated as errors.
 */
export function buildGoCalendar(
  days: GoActivityDay[],
  nowMs: number,
  weeks: number = CALENDAR_WEEKS,
): GoCalendar {
  const columnCount = Number.isFinite(weeks) ? Math.max(1, Math.floor(weeks)) : CALENDAR_WEEKS;
  const now = Number.isFinite(nowMs) ? nowMs : Date.now();
  const today = utcDateKey(now);
  const todayIndex = dayIndex(today);

  const recorded = new Map<string, GoActivityDay>();
  let recordStart: string | null = null;
  for (const day of days) {
    if (!isValidDate(day.date)) continue;
    recorded.set(day.date, day);
    if (recordStart === null || day.date < recordStart) recordStart = day.date;
  }

  // Monday-first columns: shift today back to the Monday of its week.
  const weekday = new Date(todayIndex * DAY_MS).getUTCDay();
  const mondayOffset = (weekday + 6) % 7;
  const startIndex = todayIndex - mondayOffset - (columnCount - 1) * 7;

  const weeksGrid: CalendarCell[][] = [];
  const cells: CalendarCell[] = [];
  for (let column = 0; column < columnCount; column += 1) {
    const week: CalendarCell[] = [];
    for (let row = 0; row < 7; row += 1) {
      const index = startIndex + column * 7 + row;
      const date = dateFromIndex(index);
      const record = recorded.get(date);
      const cell: CalendarCell = {
        date,
        tokens: record ? tokenCount(record) : 0,
        requests: record?.requests ?? 0,
        costMicrocents: record?.cost_microcents ?? 0,
        level: 0,
        future: index > todayIndex,
        recorded: record !== undefined,
      };
      week.push(cell);
      cells.push(cell);
    }
    weeksGrid.push(week);
  }

  // Rank the positive days into four buckets (level 1..4). Ranking by position
  // instead of raw thresholds keeps one huge day from flattening every other
  // cell, and lets a short history still reach the deepest shade.
  const sorted = cells
    .filter((cell) => !cell.future && cell.tokens > 0)
    .map((cell) => cell.tokens)
    .sort((left, right) => left - right);
  const rankByTokens = new Map<number, number>();
  sorted.forEach((tokens, index) => rankByTokens.set(tokens, index));
  for (const cell of cells) {
    if (cell.future || cell.tokens <= 0) continue;
    const rank = rankByTokens.get(cell.tokens) ?? 0;
    const share = sorted.length <= 1 ? 1 : rank / (sorted.length - 1);
    cell.level = 1 + Math.min(3, Math.floor(share * 4));
  }

  // Month captions sit above the column that contains the 1st. The first
  // column always gets one, so a window that starts mid-month is still labeled.
  const monthByColumn = new Map<number, string>();
  for (let column = 0; column < columnCount; column += 1) {
    for (const cell of weeksGrid[column]) {
      if (cell.date.slice(8, 10) === "01") {
        monthByColumn.set(column, MONTH_LABELS[Number(cell.date.slice(5, 7)) - 1]);
      }
    }
  }
  if (!monthByColumn.has(0)) {
    const firstMonth = Number(dateFromIndex(startIndex).slice(5, 7));
    monthByColumn.set(0, MONTH_LABELS[firstMonth - 1]);
  }
  const months = [...monthByColumn.entries()]
    .sort((left, right) => left[0] - right[0])
    .map(([column, label]) => ({ column, label }));

  const visible = cells.filter((cell) => !cell.future);
  const totalTokens = visible.reduce((sum, cell) => sum + cell.tokens, 0);
  const totalRequests = visible.reduce((sum, cell) => sum + cell.requests, 0);
  const totalCostMicrocents = visible.reduce((sum, cell) => sum + cell.costMicrocents, 0);
  const activeDays = visible.filter((cell) => cell.tokens > 0).length;
  const best = visible.reduce<CalendarCell | null>(
    (current, cell) => (cell.tokens > (current?.tokens ?? 0) ? cell : current),
    null,
  );

  let longestStreak = 0;
  let running = 0;
  for (const cell of visible) {
    running = cell.tokens > 0 ? running + 1 : 0;
    longestStreak = Math.max(longestStreak, running);
  }

  // The current streak ends today; while today has no tokens yet it ends
  // yesterday, so an active streak is not reported as broken every morning.
  let currentStreak = 0;
  let cursor = visible.length - 1;
  if (cursor >= 0 && visible[cursor].tokens <= 0) cursor -= 1;
  while (cursor >= 0 && visible[cursor].tokens > 0) {
    currentStreak += 1;
    cursor -= 1;
  }

  return {
    weeks: weeksGrid,
    months,
    empty: activeDays === 0,
    stats: {
      totalTokens,
      totalRequests,
      totalCostMicrocents,
      activeDays,
      currentStreak,
      longestStreak,
      best: best && best.tokens > 0 ? best : null,
      recordStart,
      windowStart: dateFromIndex(startIndex),
      today,
    },
  };
}
