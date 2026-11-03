import { describe, expect, it } from "vitest";
import {
  buildGoSeries,
  formatAxisDate,
  formatGoDate,
  modelDisplayName,
  modelShare,
} from "./goUsage";
import type { GoActivityDay } from "../types/models";

const DAY = 86_400_000;
const START = Date.UTC(2026, 9, 26);
const END = START + 10 * DAY;

function day(offset: number, cost = 10, requests = 1): GoActivityDay {
  return {
    date: new Date(START + offset * DAY).toISOString().slice(0, 10),
    cost_microcents: cost,
    requests,
    tokens: 100,
  };
}

describe("buildGoSeries", () => {
  it("scales daily cost into the monthly percentage and extrapolates", () => {
    const series = buildGoSeries(
      [day(0), day(1), day(2), day(3), day(4)],
      50,
      START,
      END,
      START + 5 * DAY,
    );

    expect(series.estimated).toBe(false);
    expect(series.totalCost).toBe(50);
    expect(series.forecast).toBe(100);
    expect(series.perDay).toBe(10);
    expect(series.runsOut).toBeNull();
    expect(series.points).toHaveLength(6);
    expect(series.points[0]).toEqual({ x: 0, value: 0 });
    expect(series.points[1].value).toBe(10);
    expect(series.points[1].dailyValue).toBe(10);
    expect(series.points[5].value).toBe(50);
  });

  it("computes the date a faster pace crosses the limit", () => {
    const series = buildGoSeries([], 60, START, END, START + 5 * DAY);
    expect(series.forecast).toBe(120);
    expect(series.runsOut).not.toBeNull();
    expect(series.runsOut!).toBeGreaterThan(START + 7 * DAY);
    expect(series.runsOut!).toBeLessThan(START + 9 * DAY);
  });

  it("extends the line to now when the daily rollup lags", () => {
    const series = buildGoSeries(
      [day(0), day(1), day(2)],
      50,
      START,
      END,
      START + 5 * DAY,
    );
    const last = series.points[series.points.length - 1];
    expect(last).toMatchObject({ x: 0.5, value: 50 });
  });

  it("falls back to a straight line without daily data", () => {
    const series = buildGoSeries([], 50, START, END, START + 5 * DAY);
    expect(series.estimated).toBe(true);
    expect(series.points).toEqual([
      { x: 0, value: 0 },
      { x: 0.5, value: 50 },
    ]);
  });

  it("does not extrapolate a limit that is already reached", () => {
    const series = buildGoSeries([], 100, START, END, START + 5 * DAY);
    expect(series.forecast).toBeNull();
    expect(series.runsOut).toBeNull();
  });

  it("waits a full day before forecasting", () => {
    const series = buildGoSeries([], 10, START, END, START + DAY / 2);
    expect(series.forecast).toBeNull();
    expect(series.perDay).toBeNull();
  });

  it("does not extrapolate after the period has ended", () => {
    const series = buildGoSeries([], 25, START, END, END + DAY);
    expect(series.forecast).toBeNull();
    expect(series.runsOut).toBeNull();
  });

  it("marks zero usage as empty", () => {
    const series = buildGoSeries([], 0, START, END, START + 5 * DAY);
    expect(series.empty).toBe(true);
    expect(series.forecast).toBeNull();
  });
});

describe("modelShare", () => {
  it("matches the Console's percentage labels", () => {
    expect(modelShare(45, 100)).toBe("45%");
    expect(modelShare(99, 100)).toBe("99%");
    expect(modelShare(0.5, 100)).toBe("<1%");
    expect(modelShare(0, 100)).toBe("0%");
    expect(modelShare(1, 0)).toBe("0%");
  });
});

describe("modelDisplayName", () => {
  it("maps known Go models and falls back to a readable id", () => {
    expect(modelDisplayName("deepseek-v4.1-flash")).toBe("DeepSeek V4.1 Flash");
    expect(modelDisplayName("glm-5.3")).toBe("GLM-5.3");
    expect(modelDisplayName("custom-model")).toBe("Custom Model");
  });
});

describe("date labels", () => {
  it("uses UTC month/day labels", () => {
    expect(formatGoDate(Date.UTC(2026, 10, 26))).toBe("11 月 26 日");
    expect(formatAxisDate("2026-10-26")).toBe("10/26");
    expect(formatAxisDate("bad")).toBe("bad");
  });
});
