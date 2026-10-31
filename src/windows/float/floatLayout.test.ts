// Unit tests for the pure floating-window layout helpers.

import { describe, expect, it } from "vitest";
import {
  DOCKED_CHROME_LENGTH,
  DOCK_HOVER_RING_COUNT,
  DOCK_HOVER_RING_RADIUS,
  MIN_FILL_PERCENT,
  clampPercent,
  compactNumber,
  dockedEdge,
  dockedWindowLength,
  dockHoverWindows,
  highestWindow,
  progressWidth,
  resetSeconds,
  ringCircumference,
  ringDashOffset,
  snapshotEqual,
} from "./floatLayout";
import type { FloatStateDto, UsageReport, WindowReport } from "../../types/models";

function window(percent: number, label = "weekly", key = label): WindowReport {
  return {
    key,
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
  dockHover: false,
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

describe("dockedWindowLength", () => {
  it("adds the strip chrome and rounds the content up", () => {
    expect(dockedWindowLength(200)).toBe(200 + DOCKED_CHROME_LENGTH + 2);
    expect(dockedWindowLength(200.2)).toBe(201 + DOCKED_CHROME_LENGTH + 2);
  });

  it("refuses to size a strip with no measurable content", () => {
    expect(dockedWindowLength(0)).toBeNull();
    expect(dockedWindowLength(-10)).toBeNull();
    expect(dockedWindowLength(Number.NaN)).toBeNull();
    expect(dockedWindowLength(Number.POSITIVE_INFINITY)).toBeNull();
  });
});

describe("dockedEdge", () => {
  it("maps every docked presentation to its edge", () => {
    expect(dockedEdge("docked_top")).toBe("top");
    expect(dockedEdge("docked_left")).toBe("left");
    expect(dockedEdge("docked_right")).toBe("right");
  });

  it("returns null for the configured layouts", () => {
    expect(dockedEdge("full")).toBeNull();
    expect(dockedEdge("compact")).toBeNull();
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

describe("dockHoverWindows", () => {
  it("returns no rings before the first report", () => {
    expect(dockHoverWindows(null)).toEqual([]);
  });

  it("orders the three OpenCode windows as 5h / weekly / monthly", () => {
    const report: UsageReport = {
      plan_type: "",
      fetched_at: 0,
      windows: [
        window(19, "近一月", "monthly"),
        window(78, "5 小时", "rolling-5h"),
        window(52, "近一周", "weekly"),
      ],
    };
    expect(dockHoverWindows(report).map((slot) => slot.key)).toEqual([
      "rolling-5h",
      "weekly",
      "monthly",
    ]);
  });

  it("fills the spare rings with extra windows and caps at three", () => {
    const report: UsageReport = {
      plan_type: "",
      fetched_at: 0,
      windows: [
        window(52, "近一周", "weekly"),
        window(30, "自定义配额", "custom"),
        window(10, "第四个", "extra"),
        window(5, "第五个", "fifth"),
      ],
    };
    const rings = dockHoverWindows(report);
    expect(rings).toHaveLength(DOCK_HOVER_RING_COUNT);
    expect(rings.map((slot) => slot.key)).toEqual(["weekly", "custom", "extra"]);
  });

  it("omits missing windows instead of reserving an empty ring", () => {
    const report: UsageReport = {
      plan_type: "",
      fetched_at: 0,
      windows: [window(52, "近一周", "weekly")],
    };
    expect(dockHoverWindows(report).map((slot) => slot.key)).toEqual(["weekly"]);
  });
});

describe("ringDashOffset", () => {
  const circumference = ringCircumference(DOCK_HOVER_RING_RADIUS);

  it("draws the used percentage as an arc", () => {
    expect(ringDashOffset(100)).toBeCloseTo(0);
    expect(ringDashOffset(50)).toBeCloseTo(circumference / 2);
    expect(ringDashOffset(0)).toBeCloseTo(circumference);
  });

  it("keeps a visible arc for a barely used window", () => {
    const offset = ringDashOffset(0.1);
    expect(offset).toBeLessThan(circumference);
    expect(circumference - offset).toBeCloseTo((MIN_FILL_PERCENT / 100) * circumference);
  });

  it("clamps out-of-range percentages instead of wrapping", () => {
    expect(ringDashOffset(-10)).toBeCloseTo(circumference);
    expect(ringDashOffset(150)).toBeCloseTo(0);
  });

  it("scales with an explicit radius", () => {
    const small = ringDashOffset(50, 10);
    expect(small).toBeCloseTo(Math.PI * 10);
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
    const a = state({ presentationMode: "docked_top" });
    expect(snapshotEqual(a, { ...a })).toBe(true);
    expect(snapshotEqual(null, null)).toBe(true);
  });

  it("detects any drift, including a different dock edge", () => {
    const a = state({ presentationMode: "docked_top" });
    expect(snapshotEqual(a, state({ presentationMode: "docked_left" }))).toBe(false);
    expect(snapshotEqual(a, state({ presentationMode: "full" }))).toBe(false);
    expect(
      snapshotEqual(a, state({ presentationMode: "docked_top", configuredMode: "compact" })),
    ).toBe(false);
    expect(snapshotEqual(a, state({ presentationMode: "docked_top", open: false }))).toBe(false);
    expect(
      snapshotEqual(a, state({ presentationMode: "docked_top", dockHover: true })),
      "the hover card is a presentation change too",
    ).toBe(false);
    expect(snapshotEqual(a, null)).toBe(false);
  });
});
