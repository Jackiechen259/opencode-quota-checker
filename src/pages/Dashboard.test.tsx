import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Dashboard } from "./Dashboard";
import type { UsageReport } from "../types/models";

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

describe("Dashboard", () => {
  it("renders the overview hero and every quota window", () => {
    render(
      <Dashboard report={report} loading={false} error={null} nowMs={1_778_800_000_000} onRefresh={() => {}} />,
    );
    expect(screen.getByText("最高负载")).toBeInTheDocument();
    expect(screen.getByText("78.0%")).toBeInTheDocument();
    expect(screen.getAllByText("5 小时").length).toBeGreaterThan(0);
    expect(screen.getByText("近一周")).toBeInTheDocument();
    expect(screen.getByText("共 2 个窗口")).toBeInTheDocument();
  });

  it("shows the loading skeleton while fetching", () => {
    render(<Dashboard report={null} loading error={null} nowMs={0} onRefresh={() => {}} />);
    expect(screen.getByText("正在安全地加载用量数据…")).toBeInTheDocument();
  });

  it("shows the empty state before the first report", () => {
    render(<Dashboard report={null} loading={false} error={null} nowMs={0} onRefresh={() => {}} />);
    expect(screen.getByText("还没有配额数据")).toBeInTheDocument();
  });

  it("shows the error notice without losing the previous report", () => {
    render(
      <Dashboard
        report={report}
        loading={false}
        error={{ code: "rate_limited", user: "请求过于频繁，请稍后重试。", detail: "429" }}
        nowMs={1_778_800_000_000}
        onRefresh={() => {}}
      />,
    );
    expect(screen.getByText(/暂时无法更新配额数据/)).toBeInTheDocument();
    expect(screen.getByText("最高负载")).toBeInTheDocument();
  });

  it("refresh button triggers the callback", () => {
    const onRefresh = vi.fn();
    render(<Dashboard report={null} loading={false} error={null} nowMs={0} onRefresh={onRefresh} />);
    screen.getByRole("button", { name: "立即刷新" }).click();
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });
});
