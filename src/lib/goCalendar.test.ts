import { describe, expect, it } from "vitest";
import { CALENDAR_WEEKS, buildGoCalendar, utcDateKey } from "./goCalendar";
import type { GoActivityDay } from "../types/models";

const DAY = 86_400_000;
/** Wednesday 2026-11-11, 05:00 UTC. */
const NOW = Date.UTC(2026, 10, 11, 5);

function day(date: string, tokens: number, requests = 1): GoActivityDay {
  return {
    date,
    cost_microcents: tokens * 50,
    requests,
    tokens,
  };
}

function dates(count: number, start = "2026-11-01"): string[] {
  const base = Date.parse(`${start}T00:00:00Z`);
  return Array.from({ length: count }, (_, index) =>
    new Date(base + index * DAY).toISOString().slice(0, 10),
  );
}

describe("buildGoCalendar", () => {
  it("anchors a Monday-first 53-week grid on the current week", () => {
    const calendar = buildGoCalendar([], NOW);

    expect(calendar.weeks).toHaveLength(CALENDAR_WEEKS);
    expect(calendar.weeks.every((week) => week.length === 7)).toBe(true);
    const lastWeek = calendar.weeks[CALENDAR_WEEKS - 1];
    expect(lastWeek[0].date).toBe("2026-11-09"); // Monday of this week
    expect(lastWeek[2].date).toBe("2026-11-11"); // today (Wednesday)
    expect(lastWeek[2].future).toBe(false);
    expect(lastWeek[3].future).toBe(true);
    expect(calendar.weeks[0][0].date).toBe("2025-11-10");
    expect(calendar.stats.windowStart).toBe("2025-11-10");
    expect(calendar.stats.today).toBe("2026-11-11");
    expect(calendar.empty).toBe(true);
  });

  it("ranks positive days into four blue levels", () => {
    const history = dates(8).map((date, index) => day(date, (index + 1) * 100));
    const calendar = buildGoCalendar(history, NOW);

    const levels = calendar.weeks
      .flat()
      .filter((cell) => cell.recorded)
      .map((cell) => cell.level);
    expect(levels).toEqual([1, 1, 2, 2, 3, 3, 4, 4]);
    expect(calendar.stats.totalTokens).toBe(3_600);
    expect(calendar.stats.activeDays).toBe(8);
  });

  it("ignores future days in the totals, the ranking, and hover detail", () => {
    const calendar = buildGoCalendar(
      [day("2026-11-10", 100), day("2026-11-12", 9_999)],
      NOW,
    );

    const tomorrow = calendar.weeks[CALENDAR_WEEKS - 1][3];
    expect(tomorrow.date).toBe("2026-11-12");
    expect(tomorrow.future).toBe(true);
    expect(tomorrow.level).toBe(0);
    expect(tomorrow.recorded).toBe(true);
    expect(calendar.stats.totalTokens).toBe(100);
    expect(calendar.stats.activeDays).toBe(1);
  });

  it("computes streaks, best day, and the record start", () => {
    const calendar = buildGoCalendar(
      [
        day("2024-05-01", 10), // older than the window, still the record start
        day("2026-11-07", 30),
        day("2026-11-08", 90),
        day("2026-11-09", 20),
        day("2026-11-10", 40),
      ],
      NOW,
    );

    expect(calendar.stats.longestStreak).toBe(4);
    // Today is empty, so the current streak ends yesterday.
    expect(calendar.stats.currentStreak).toBe(4);
    expect(calendar.stats.best?.date).toBe("2026-11-08");
    expect(calendar.stats.recordStart).toBe("2024-05-01");
  });

  it("counts today when it already has usage", () => {
    const calendar = buildGoCalendar(
      [day("2026-11-10", 40), day("2026-11-11", 60)],
      NOW,
    );
    expect(calendar.stats.currentStreak).toBe(2);
    expect(calendar.stats.longestStreak).toBe(2);
  });

  it("ignores malformed and impossible dates", () => {
    const calendar = buildGoCalendar(
      [
        day("not-a-date", 5),
        day("2026-02-30", 5),
        day("2026-11-11", 1_000),
      ],
      NOW,
    );

    const recorded = calendar.weeks.flat().filter((cell) => cell.recorded);
    expect(recorded).toHaveLength(1);
    expect(recorded[0].date).toBe("2026-11-11");
    expect(calendar.stats.recordStart).toBe("2026-11-11");
  });

  it("labels every month that starts inside the grid plus the first column", () => {
    const calendar = buildGoCalendar([], NOW);

    expect(calendar.months[0]).toEqual({ column: 0, label: "11月" });
    const labels = calendar.months.map((month) => month.label);
    expect(labels).toContain("12月");
    expect(labels).toContain("1月");
    // A 53-week grid reaches back into the first month, so it may be labeled
    // twice (start and end), but the columns are always strictly increasing.
    expect(
      calendar.months.every(
        (month, index) =>
          index === 0 || month.column > calendar.months[index - 1].column,
      ),
    ).toBe(true);
  });
});

describe("utcDateKey", () => {
  it("formats the UTC calendar date regardless of the local clock", () => {
    expect(utcDateKey(NOW)).toBe("2026-11-11");
    expect(utcDateKey(Date.UTC(2026, 0, 1, 0, 0, 0))).toBe("2026-01-01");
  });
});
