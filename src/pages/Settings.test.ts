import { describe, expect, it } from "vitest";
import { validateMonitorForm } from "./Settings";
import { quotaHealth, type WindowReport } from "../types/models";

describe("validateMonitorForm", () => {
  it("accepts boundary values", () => {
    expect(validateMonitorForm("30", "0", "100", "85")).toBeNull();
    expect(validateMonitorForm("3600", "80", "85", "85")).toBeNull();
  });

  it("rejects invalid intervals", () => {
    expect(validateMonitorForm("29", "80", "85", "85")).toMatch(/30–3600/);
    expect(validateMonitorForm("3601", "80", "85", "85")).toMatch(/30–3600/);
    expect(validateMonitorForm("abc", "80", "85", "85")).toMatch(/30–3600/);
    expect(validateMonitorForm("80.5", "80", "85", "85")).toMatch(/30–3600/);
  });

  it("rejects invalid thresholds", () => {
    expect(validateMonitorForm("300", "-1", "85", "85")).toMatch(/0–100/);
    expect(validateMonitorForm("300", "80", "101", "85")).toMatch(/0–100/);
    expect(validateMonitorForm("300", "80", "85", "NaN")).toMatch(/0–100/);
  });
});

describe("quotaHealth", () => {
  it("buckets at the boundaries", () => {
    expect(quotaHealth(69.9)).toBe("healthy");
    expect(quotaHealth(70)).toBe("warning");
    expect(quotaHealth(89.9)).toBe("warning");
    expect(quotaHealth(90)).toBe("critical");
  });
});

describe("WindowReport reset math", () => {
  it("counts down from reset time", () => {
    const window: WindowReport = {
      key: "weekly",
      label: "近一周",
      quota: 100,
      used: 52,
      remaining: 48,
      percent: 52,
      subscribe_time: 1_778_800_000_000,
      reset_time: 1_778_806_132_000,
      reset_in_secs: 6_132,
    };
    const nowMs = 1_778_800_000_000;
    expect(Math.floor((window.reset_time - nowMs) / 1_000)).toBe(6_132);
  });
});
