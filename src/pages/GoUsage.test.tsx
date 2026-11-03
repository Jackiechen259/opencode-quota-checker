import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
// Registers the Tauri mocks before the page (and its api import) loads.
import "../test/tauri";
import type {
  GoOverviewDto,
  TokenUsageDto,
  UsageReport,
  WindowReport,
} from "../types/models";

const mockState = vi.hoisted(() => ({
  data: null as GoOverviewDto | null,
}));

vi.mock("../hooks/useApp", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../hooks/useApp")>();
  return {
    ...actual,
    useGoOverview: () => ({
      data:
        mockState.data ??
        ({
          status: null,
          activity: null,
          billing: null,
          statusError: null,
          activityError: null,
          billingError: null,
          loading: false,
          lastSuccessMs: null,
        } satisfies GoOverviewDto),
      refresh: () => {},
      reload: () => {},
    }),
  };
});

import { GoUsagePage } from "./GoUsage";

const DAY = 86_400_000;
const START = Date.UTC(2026, 9, 26);
const END = Date.UTC(2026, 10, 26);
const NOW = START + 7.75 * DAY;

const report: UsageReport = {
  plan_type: "",
  fetched_at: NOW,
  windows: [
    {
      key: "rolling-5h",
      label: "5 小时",
      quota: 100,
      used: 4,
      remaining: 96,
      percent: 4,
      subscribe_time: NOW,
      reset_time: NOW + 3 * 3_600_000,
      reset_in_secs: 10_800,
      limited: false,
    },
    {
      key: "weekly",
      label: "近一周",
      quota: 100,
      used: 7,
      remaining: 93,
      percent: 7,
      subscribe_time: NOW,
      reset_time: NOW + 5 * DAY,
      reset_in_secs: 5 * 86_400,
      limited: false,
    },
    {
      key: "monthly",
      label: "近一月",
      quota: 100,
      used: 24,
      remaining: 76,
      percent: 24,
      subscribe_time: NOW,
      reset_time: END,
      reset_in_secs: 23 * 86_400,
      limited: false,
    },
  ],
};

const token: TokenUsageDto = {
  report: {
    range: "30d",
    requests: 8_300,
    input_tokens: 26_000_000,
    output_tokens: 6_000_000,
    cache_read_tokens: 1_000_000_000,
    cache_write_tokens: 0,
    total_tokens: 1_032_000_000,
    cost_microcents: 1_400_000_000,
    fetched_at: NOW,
  },
  loading: false,
  error: null,
  lastSuccessMs: NOW,
};

const quotaWindow = (key: string): WindowReport =>
  report.windows.find((window) => window.key === key)!;

const fullOverview: GoOverviewDto = {
  status: {
    product: "go",
    cancel_at_period_end: false,
    use_balance: false,
    access: {
      starts_at: START,
      five_hour: {
        used_microcents: 4,
        limit_microcents: 100,
        resets_at: NOW + 3 * 3_600_000,
        percent: 4,
        limited: false,
      },
      week: {
        used_microcents: 7,
        limit_microcents: 100,
        resets_at: NOW + 5 * DAY,
        percent: 7,
        limited: false,
      },
      month: {
        used_microcents: 24,
        limit_microcents: 100,
        resets_at: END,
        percent: 24,
        limited: false,
      },
    },
    fetched_at: NOW,
  },
  activity: {
    starts_at: START,
    daily: [
      {
        date: "2026-10-26",
        cost_microcents: 50,
        requests: 60,
        tokens: 2_000,
      },
      {
        date: "2026-10-27",
        cost_microcents: 50,
        requests: 40,
        tokens: 2_000,
      },
    ],
    models: [
      { provider: "opencode", model: "deepseek-v4.1-flash", requests: 99 },
      { provider: "opencode", model: "glm-5.3", requests: 1 },
    ],
    total_cost_microcents: 100,
    total_requests: 100,
    total_tokens: 4_000,
    fetched_at: NOW,
  },
  billing: {
    available_microcents: 0,
    balance_microcents: 0,
    billing_mode: "prepaid",
    fetched_at: NOW,
  },
  statusError: null,
  activityError: null,
  billingError: null,
  loading: false,
  lastSuccessMs: NOW,
};

const emptyOverview: GoOverviewDto = {
  status: null,
  activity: null,
  billing: null,
  statusError: null,
  activityError: null,
  billingError: null,
  loading: false,
  lastSuccessMs: null,
};

function renderPage(overrides: Partial<Parameters<typeof GoUsagePage>[0]> = {}) {
  return render(
    <GoUsagePage
      report={report}
      loading={false}
      token={token}
      nowMs={NOW}
      onRefresh={() => {}}
      {...overrides}
    />,
  );
}

describe("GoUsagePage", () => {
  beforeEach(() => {
    mockState.data = fullOverview;
  });

  it("renders the monthly summary, forecast, limits, models, and extra usage", () => {
    renderPage();

    expect(
      screen.getByRole("heading", { name: "Go 用量", level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByText("24%")).toBeInTheDocument();
    expect(screen.getByText(/11 月 26 日 续订/)).toBeInTheDocument();
    expect(screen.getByText(/预计 96%/)).toBeInTheDocument();
    expect(screen.getByText("平均每天 3.1%")).toBeInTheDocument();
    expect(screen.getByText("4% 已用")).toBeInTheDocument();
    expect(screen.getByText("7% 已用")).toBeInTheDocument();
    expect(screen.getByText("DeepSeek V4.1 Flash")).toBeInTheDocument();
    expect(screen.getByText("99%")).toBeInTheDocument();
    expect(screen.getByText("Requests")).toBeInTheDocument();
    expect(screen.getByText("100")).toBeInTheDocument();
    expect(screen.getByText("4,000")).toBeInTheDocument();
    expect(screen.getByText("已关闭 · 达到限额后暂停请求")).toBeInTheDocument();
    expect(screen.getByText("$0.00")).toBeInTheDocument();
    expect(screen.getByText("在 Console 打开")).toBeInTheDocument();
  });

  it("marks an exhausted month as critical", () => {
    mockState.data = {
      ...fullOverview,
      status: {
        ...fullOverview.status!,
        access: {
          ...fullOverview.status!.access!,
          month: {
            used_microcents: 100,
            limit_microcents: 100,
            resets_at: END,
            percent: 100,
            limited: true,
          },
        },
      },
    };
    renderPage();

    expect(screen.getAllByText("100%").length).toBeGreaterThan(0);
    expect(screen.getByText(/本月额度已用尽/)).toBeInTheDocument();
  });

  it("falls back to the quota report when status and activity are unavailable", () => {
    mockState.data = {
      ...emptyOverview,
      statusError: {
        code: "usage_permission_denied",
        user: "无法读取工作区用量。",
        detail: "403",
      },
      activityError: {
        code: "usage_permission_denied",
        user: "无法读取工作区用量。",
        detail: "403",
      },
    };
    renderPage();

    expect(screen.getByText(/订阅状态暂不可用/)).toBeInTheDocument();
    expect(screen.getByText(/Go 明细暂不可用/)).toBeInTheDocument();
    expect(screen.getByText(/请求\/Token 回退为近 30 天/)).toBeInTheDocument();
    expect(screen.getByText("8,300")).toBeInTheDocument();
    expect(screen.getByText("4% 已用")).toBeInTheDocument();
  });

  it("shows a skeleton while the first overview request is in flight", () => {
    mockState.data = { ...emptyOverview, loading: true };
    renderPage({ report: null, loading: true });
    expect(screen.getByLabelText("正在加载 Go 用量")).toBeInTheDocument();
  });

  it("shows an empty state when there is no data", () => {
    mockState.data = emptyOverview;
    renderPage({ report: null, loading: false });
    expect(screen.getByText("还没有 Go 用量数据")).toBeInTheDocument();
  });

  it("opens the Console Go page from the header", () => {
    renderPage();
    expect(screen.getByRole("button", { name: "在 Console 打开" })).toBeInTheDocument();
    expect(quotaWindow("monthly").percent).toBe(24);
  });
});
