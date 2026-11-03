import { describe, expect, it } from "vitest";
import {
  bytes,
  compact,
  countdown,
  countdownShort,
  lastCheckedText,
  number,
  percent,
  relative,
  timestamp,
  usdFromMicrocents,
} from "./format";

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
  it("uses thousands separators and at most one decimal", () => {
    expect(number(75780.9)).toBe("75,780.9");
    expect(number(52.34)).toBe("52.3");
    expect(number(100000)).toBe("100,000");
    expect(number(0)).toBe("0");
  });
});

describe("percent", () => {
  it("formats whole percentages without a decimal part", () => {
    expect(percent(78)).toBe("78%");
    expect(percent(52.34)).toBe("52%");
    expect(percent(52.5)).toBe("53%");
    expect(percent(0.4)).toBe("0%");
    expect(percent(99.9)).toBe("100%");
  });

  it("tolerates values over 100", () => {
    expect(percent(120)).toBe("120%");
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
  it("scales units without a redundant trailing zero", () => {
    expect(bytes(512)).toBe("512 B");
    expect(bytes(2048)).toBe("2 KB");
    expect(bytes(1536)).toBe("1.5 KB");
    expect(bytes(5 * 1024 * 1024)).toBe("5 MB");
  });
});

describe("compact", () => {
  it("shortens large token totals with Chinese units and keeps small values exact", () => {
    expect(compact(0)).toBe("0");
    expect(compact(999)).toBe("999");
    expect(compact(1_000)).toBe("1,000");
    expect(compact(8_534)).toBe("8,534");
    expect(compact(10_000)).toBe("1万");
    expect(compact(26_017_766)).toBe("2601.78万");
    expect(compact(1_072_476_433)).toBe("10.72亿");
    expect(compact(1_260_000_000)).toBe("12.6亿");
    expect(compact(Number.NaN)).toBe("—");
  });
});

describe("usdFromMicrocents", () => {
  it("converts microcents with two decimals", () => {
    expect(usdFromMicrocents(0)).toBe("$0.00");
    expect(usdFromMicrocents(30_000_000)).toBe("$0.30");
    expect(usdFromMicrocents(1_388_554_159)).toBe("$13.89");
    expect(usdFromMicrocents(Number.NaN)).toBe("—");
  });
});
