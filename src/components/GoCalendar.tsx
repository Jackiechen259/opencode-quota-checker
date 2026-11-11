// GitHub-style year heatmap for daily Go usage.
//
// One cell per UTC day across 53 week columns; the blue level is the day's rank
// by token count (four buckets, so one huge day does not flatten the rest).
// Hovering a cell reports that day above the legend. The grid is exposed as a
// single image to assistive technology; the per-day numbers live in the cell
// titles and in the hover detail line.

import { useMemo, useState } from "react";
import { compact, number, usdFromMicrocents } from "../lib/format";
import { buildGoCalendar } from "../lib/goCalendar";
import { formatGoDate } from "../lib/goUsage";
import type { GoActivityDay } from "../types/models";

/** Weekday captions, Monday-first rows (一 / 三 / 五 = rows 0 / 2 / 4). */
const WEEKDAY_LABELS = [
  { row: 0, label: "一" },
  { row: 2, label: "三" },
  { row: 4, label: "五" },
];

const LEGEND_LEVELS = [0, 1, 2, 3, 4];

/** `2026-11-11` → `11 月 11 日`; older years keep the year for clarity. */
function dayLabel(date: string, today: string): string {
  const ms = Date.parse(`${date}T00:00:00Z`);
  if (!Number.isFinite(ms)) return date;
  const label = formatGoDate(ms);
  return date.slice(0, 4) === today.slice(0, 4) ? label : `${date.slice(0, 4)} 年 ${label}`;
}

interface Props {
  /** Locally accumulated daily history from the Go overview DTO. */
  days: GoActivityDay[];
  /** Clock of the page; the grid is anchored to this instant. */
  nowMs: number;
}

export function GoCalendar({ days, nowMs }: Props) {
  const calendar = useMemo(() => buildGoCalendar(days, nowMs), [days, nowMs]);
  const [activeDate, setActiveDate] = useState<string | null>(null);
  const active = useMemo(() => {
    if (activeDate === null) return null;
    for (const week of calendar.weeks) {
      for (const cell of week) {
        if (cell.date === activeDate) return cell;
      }
    }
    return null;
  }, [activeDate, calendar]);

  const { stats } = calendar;
  const detail = active ? (
    <>
      <span>{dayLabel(active.date, stats.today)}</span>
      <span>{compact(active.tokens)} Token</span>
      <span>{number(active.requests)} 请求</span>
      <span>{usdFromMicrocents(active.costMicrocents)}</span>
    </>
  ) : (
    <span>
      {calendar.empty
        ? "还没有记录到 Go 用量；打开此页即可开始累积。"
        : "悬停格子查看当天明细"}
    </span>
  );
  const recordNote =
    stats.recordStart !== null && stats.recordStart > stats.windowStart
      ? `本地记录从 ${dayLabel(stats.recordStart, stats.today)} 开始，更早的格子显示为空。`
      : null;

  return (
    <div className="go-calendar">
      <div
        className="go-calendar-grid"
        role="img"
        aria-label={`过去一年 Go 用量日历：${stats.activeDays} 天有用量，共 ${compact(
          stats.totalTokens,
        )} Token`}
        onMouseLeave={() => setActiveDate(null)}
      >
        {calendar.months.map((month) => (
          <span
            key={`${month.column}-${month.label}`}
            className="go-calendar-month"
            style={{ gridColumn: month.column + 2, gridRow: 1 }}
          >
            {month.label}
          </span>
        ))}
        {WEEKDAY_LABELS.map((weekday) => (
          <span
            key={weekday.label}
            className="go-calendar-weekday"
            style={{ gridColumn: 1, gridRow: weekday.row + 2 }}
          >
            {weekday.label}
          </span>
        ))}
        {calendar.weeks.map((week, column) =>
          week.map((cell, row) => (
            <span
              key={cell.date}
              className="go-calendar-cell"
              data-level={cell.level}
              data-future={cell.future ? "true" : undefined}
              style={{ gridColumn: column + 2, gridRow: row + 2 }}
              title={
                cell.future
                  ? undefined
                  : `${cell.date} · ${compact(cell.tokens)} Token · ${number(
                      cell.requests,
                    )} 请求`
              }
              onMouseEnter={() => {
                if (!cell.future) setActiveDate(cell.date);
              }}
            />
          )),
        )}
      </div>

      <div className="go-calendar-stats">
        <div>
          <span>近一年 Token</span>
          <strong>{compact(stats.totalTokens)}</strong>
        </div>
        <div>
          <span>活跃天数</span>
          <strong>{stats.activeDays} 天</strong>
        </div>
        <div>
          <span>最长连续</span>
          <strong>{stats.longestStreak} 天</strong>
        </div>
        <div>
          <span>最高单日</span>
          <strong title={stats.best ? stats.best.date : undefined}>
            {compact(stats.best?.tokens ?? 0)}
          </strong>
        </div>
      </div>

      <div className="go-calendar-footer">
        <div className="go-calendar-detail">{detail}</div>
        <div className="go-calendar-legend" aria-hidden="true">
          <span>少</span>
          {LEGEND_LEVELS.map((level) => (
            <i key={level} className="go-calendar-swatch" data-level={level} />
          ))}
          <span>多</span>
        </div>
      </div>

      {recordNote ? <p className="go-muted">{recordNote}</p> : null}
    </div>
  );
}
