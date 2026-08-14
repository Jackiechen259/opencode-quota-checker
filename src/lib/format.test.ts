import { describe, expect, it } from "vitest";
import { bytes, countdown, countdownShort, lastCheckedText, number, percent, relative, timestamp } from "./format";

describe("countdown", () => {
  it("handles boundaries", () => {
    expect(countdown(-1)).toBe("即将重置");
    expect(countdown(0)).toBe("即将重置");
    expect(countdown(59)).toBe("0 分 59 秒");
    expect(countdown(3_661)).toBe("1 小时 1 分");
    expect(countdown(90_000)).toBe("1 天 1 小时");
  });
});

describe("countdownShort", () => {
  it("formats compact durations", () => {
    expect(countdownShort(0)).toBe("即将重置");
    expect(countdownShort(3_661)).toBe("01:01");
    expect(countdownShort(90_000)).toBe("1天 1时");
  });
});

describe("timestamp", () => {
  it("formats valid timestamps and rejects invalid ones", () => {
    expect(timestamp(0)).toBe("1970-01-01 00:00:00 UTC");
    expect(timestamp(Number.MAX_SAFE_INTEGER)).toBe("未知时间");
  });
});

describe("number", () => {
  it("uses thousands separators and one decimal", () => {
    expect(number(75780.9)).toBe("75,780.9");
    expect(number(100000)).toBe("100,000.0");
    expect(number(0)).toBe("0.0");
  });
});

describe("percent", () => {
  it("formats one decimal", () => {
    expect(percent(78)).toBe("78.0%");
    expect(percent(52.34)).toBe("52.3%");
  });
});

describe("relative", () => {
  it("descends gracefully", () => {
    expect(relative(0, 0)).toBe("刚刚");
    expect(relative(0, 3_000)).toBe("刚刚");
    expect(relative(0, 30_000)).toBe("30 秒前");
    expect(relative(0, 120_000)).toBe("2 分钟前");
    expect(relative(0, 3_600_000)).toBe("1 小时前");
    expect(relative(0, 172_800_000)).toBe("2 天前");
  });
});

describe("lastCheckedText", () => {
  it("handles null and fresh timestamps", () => {
    expect(lastCheckedText(null)).toBe("从未");
    expect(lastCheckedText(Date.now())).toBe("刚刚");
    expect(lastCheckedText(Date.now() - 65_000)).toBe("1 分钟前");
    expect(lastCheckedText(Date.now() - 3_700_000)).toBe("1 小时前");
  });
});

describe("bytes", () => {
  it("scales units", () => {
    expect(bytes(512)).toBe("512 B");
    expect(bytes(2048)).toBe("2.0 KB");
    expect(bytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });
});
