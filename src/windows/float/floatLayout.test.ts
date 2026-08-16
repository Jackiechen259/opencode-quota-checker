// Unit tests for the pure floating-window layout helpers.

import { describe, expect, it } from "vitest";
import {
  MIN_FILL_PERCENT,
  clampPercent,
  compactNumber,
  highestWindow,
  progressWidth,
  resetSeconds,
  snapshotEqual,
} from "./floatLayout";
import type { FloatStateDto, UsageReport, WindowReport } from "../../types/models";

function window(percent: number, label = "weekly"): WindowReport {
  return {
    key: label,
    label,
    quota: 100,
    used: percent,
    remaining: 100 - percent,
    percent,
    subscribe_time: 0,
    reset_time: 0,
    reset_in_secs: 0,
  };
}

const state = (partial: Partial<FloatStateDto>): FloatStateDto => ({
  open: true,
  configuredMode: "full",
  presentationMode: "full",
  topDocked: false,
  ...partial,
});

describe("progressWidth", () => {
  it("hides the fill at exactly 0%", () => {
    expect(progressWidth(0)).toBe(0);
    expect(progressWidth(-5)).toBe(0);
  });

  it("keeps a minimum visible width for tiny positive percentages", () => {
    expect(progressWidth(0.1)).toBe(MIN_FILL_PERCENT);
    expect(progressWidth(0.5)).toBe(MIN_FILL_PERCENT);
  });

  it("passes through mid-range percentages", () => {
    expect(progressWidth(50)).toBe(50);
    expect(progressWidth(99.9)).toBe(99.9);
  });

  it("clamps at 100% and above", () => {
    expect(progressWidth(100)).toBe(100);
    expect(progressWidth(120)).toBe(100);
  });
});

describe("clampPercent", () => {
  it("clamps into 0..100", () => {
    expect(clampPercent(-3)).toBe(0);
    expect(clampPercent(50)).toBe(50);
    expect(clampPercent(101)).toBe(100);
  });
});

describe("compactNumber", () => {
  it("formats whole remaining quota without decimals", () => {
    expect(compactNumber(960)).toBe("960");
    expect(compactNumber(1000)).toBe("1,000");
  });
});

describe("highestWindow", () => {
  it("returns the highest-risk window", () => {
    const report: UsageReport = {
      plan_type: "",
      fetched_at: 0,
      windows: [window(10, "a"), window(90, "b"), window(50, "c")],
    };
    expect(highestWindow(report)?.key).toBe("b");
  });

  it("returns null for an empty or missing report", () => {
    expect(highestWindow(null)).toBeNull();
    expect(highestWindow({ plan_type: "", fetched_at: 0, windows: [] })).toBeNull();
  });
});

describe("resetSeconds", () => {
  it("never goes negative", () => {
    expect(resetSeconds(1_000, 2_000)).toBe(0);
    expect(resetSeconds(2_000, 1_000)).toBe(1);
  });
});

describe("snapshotEqual", () => {
  it("treats identical snapshots as equal", () => {
    const a = state({ presentationMode: "docked", topDocked: true });
    expect(snapshotEqual(a, { ...a })).toBe(true);
    expect(snapshotEqual(null, null)).toBe(true);
  });

  it("detects any drift", () => {
    const a = state({ presentationMode: "docked", topDocked: true });
    expect(snapshotEqual(a, state({ presentationMode: "docked" }))).toBe(false);
    expect(snapshotEqual(a, state({ presentationMode: "full" }))).toBe(false);
    expect(snapshotEqual(a, state({ presentationMode: "docked", topDocked: true, open: false }))).toBe(
      false,
    );
    expect(snapshotEqual(a, null)).toBe(false);
  });
});
