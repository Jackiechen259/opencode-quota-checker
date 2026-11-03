import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Dashboard } from "./Dashboard";
import type { TokenUsageDto, UsageReport } from "../types/models";

const report: UsageReport = {
  plan_type: "",
  fetched_at: 1_778_800_000_000,
  windows: [
    {
      key: "rolling-5h",
      label: "5 小时",
      quota: 100,
      used: 78,
      remaining: 22,
      percent: 78,
      subscribe_time: 1_778_806_132_000,
      reset_time: 1_778_806_132_000,
      reset_in_secs: 6_132,
    },
    {
      key: "weekly",
      label: "近一周",
      quota: 100,
      used: 52,
      remaining: 48,
      percent: 52,
      subscribe_time: 1_778_800_000_000,
      reset_time: 1_778_800_000_000,
      reset_in_secs: 0,
    },
  ],
};

const noToken: TokenUsageDto = {
  report: null,
  loading: false,
  error: null,
  lastSuccessMs: null,
};

const tokenReport: TokenUsageDto = {
  report: {
    range: "30d",
    requests: 7_885,
    input_tokens: 26_017_766,
    output_tokens: 6_469_246,
    cache_read_tokens: 1_039_989_421,
    cache_write_tokens: 0,
    total_tokens: 1_072_476_433,
    cost_microcents: 1_388_554_159,
    fetched_at: 1_778_800_000_000,
  },
  loading: false,
  error: null,
  lastSuccessMs: 1_778_800_000_000,
};

describe("Dashboard", () => {
  it("renders the overview hero and every quota window", () => {
    render(
      <Dashboard
        report={report}
        loading={false}
        error={null}
        token={noToken}
        nowMs={1_778_800_000_000}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByText("最高负载")).toBeInTheDocument();
    expect(screen.getByText("78%")).toBeInTheDocument();
    expect(screen.getAllByText("5 小时").length).toBeGreaterThan(0);
    expect(screen.getByText("近一周")).toBeInTheDocument();
    expect(screen.getByText("共 2 个窗口")).toBeInTheDocument();
  });

  it("renders the workspace token card with totals, requests and cost", () => {
    render(
      <Dashboard
        report={report}
        loading={false}
        error={null}
        token={tokenReport}
        nowMs={1_778_800_000_000}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByText("Token 用量")).toBeInTheDocument();
    expect(screen.getByText("10.72亿")).toBeInTheDocument();
    expect(screen.getByText("2601.78万")).toBeInTheDocument();
    expect(screen.getByText("请求 7,885")).toBeInTheDocument();
    expect(screen.getByText(/折算费用 \$13\.89/)).toBeInTheDocument();
    expect(screen.getByText(/全部模型/)).toBeInTheDocument();
  });

  it("shows a loading skeleton before the first token report", () => {
    render(
      <Dashboard
        report={report}
        loading={false}
        error={null}
        token={{ report: null, loading: true, error: null, lastSuccessMs: null }}
        nowMs={0}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByLabelText("正在加载 Token 用量")).toBeInTheDocument();
  });

  it("shows an actionable permission hint when token usage is forbidden", () => {
    render(
      <Dashboard
        report={report}
        loading={false}
        error={null}
        token={{
          report: null,
          loading: false,
          lastSuccessMs: null,
          error: {
            code: "usage_permission_denied",
            user: "无法读取工作区 Token 用量：API Key 需要 Usage › Read 权限。",
            detail: "403",
          },
        }}
        nowMs={0}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByText(/暂时无法读取 Token 用量/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "打开 Console" })).toBeInTheDocument();
  });

  it("keeps the last token report when a refresh fails", () => {
    render(
      <Dashboard
        report={report}
        loading={false}
        error={null}
        token={{
          ...tokenReport,
          error: {
            code: "rate_limited",
            user: "请求过于频繁，请稍后重试。",
            detail: "429",
          },
        }}
        nowMs={1_778_800_000_000}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByText("10.72亿")).toBeInTheDocument();
    expect(screen.getByText(/Token 用量更新失败/)).toBeInTheDocument();
  });

  it("shows the loading skeleton while fetching", () => {
    render(
      <Dashboard
        report={null}
        loading
        error={null}
        token={noToken}
        nowMs={0}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByText("正在安全地加载用量数据…")).toBeInTheDocument();
  });

  it("shows the empty state before the first report", () => {
    render(
      <Dashboard
        report={null}
        loading={false}
        error={null}
        token={noToken}
        nowMs={0}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByText("还没有配额数据")).toBeInTheDocument();
  });

  it("shows the error notice without losing the previous report", () => {
    render(
      <Dashboard
        report={report}
        loading={false}
        error={{ code: "rate_limited", user: "请求过于频繁，请稍后重试。", detail: "429" }}
        token={noToken}
        nowMs={1_778_800_000_000}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByText(/暂时无法更新配额数据/)).toBeInTheDocument();
    expect(screen.getByText("最高负载")).toBeInTheDocument();
  });

  it("refresh button triggers the callback", () => {
    const onRefresh = vi.fn();
    render(
      <Dashboard
        report={null}
        loading={false}
        error={null}
        token={noToken}
        nowMs={0}
        onRefresh={onRefresh}
      />,
    );
    screen.getByRole("button", { name: "立即刷新" }).click();
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });
});
