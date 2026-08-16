// Floating window regression tests.
//
// The core scenario these pin: the frontend renders EXCLUSIVELY from the
// backend-delivered `presentationMode` and keeps no local mode state, so
// "Full UI inside a Compact native window" cannot happen. Also covered:
// every view, every state (loading/error/empty), extreme percentages, long
// labels, and the window-control commands.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
// Must precede the FloatWindow import: registers the Tauri mocks.
import { tauri } from "../../test/tauri";
import { EVENT, type FloatStateDto, type UsageDto, type WindowReport } from "../../types/models";
import { FloatWindow } from "./FloatWindow";

const fullState: FloatStateDto = {
  open: true,
  configuredMode: "full",
  presentationMode: "full",
  topDocked: false,
};

const compactState: FloatStateDto = {
  open: true,
  configuredMode: "compact",
  presentationMode: "compact",
  topDocked: false,
};

const dockedState: FloatStateDto = {
  open: true,
  configuredMode: "full",
  presentationMode: "docked",
  topDocked: true,
};

function windowReport(key: string, label: string, percent: number): WindowReport {
  return {
    key,
    label,
    quota: 1000,
    used: percent * 10,
    remaining: 1000 - percent * 10,
    percent,
    subscribe_time: 0,
    reset_time: Date.now() + 3_600_000,
    reset_in_secs: 3_600,
  };
}

const usage = (windows: WindowReport[]): UsageDto => ({
  report: { plan_type: "", windows, fetched_at: Date.now() },
  loading: false,
  error: null,
  lastSuccessMs: Date.now(),
});

const noUsage: UsageDto = { report: null, loading: false, error: null, lastSuccessMs: null };

const loadingUsage: UsageDto = { report: null, loading: true, error: null, lastSuccessMs: null };

const errorUsage: UsageDto = {
  report: null,
  loading: false,
  error: { code: "fetch_failed", user: "无法连接 OpenCode 服务，请检查网络后重试。", detail: "x" },
  lastSuccessMs: null,
};

function installInvokeRouter(overrides: {
  floatState?: FloatStateDto | Promise<FloatStateDto>;
  usage?: UsageDto;
} = {}) {
  tauri.invoke.mockImplementation(async (command: string) => {
    switch (command) {
      case "get_float_state":
        return overrides.floatState ?? fullState;
      case "get_usage":
        return overrides.usage ?? noUsage;
      case "set_float_mode":
      case "close_float_window":
      case "refresh_usage":
        return undefined;
      default:
        return undefined;
    }
  });
}

beforeEach(() => {
  tauri.reset();
  installInvokeRouter();
});

describe("FloatWindow state architecture", () => {
  it("shows a lightweight boot shell until the float snapshot resolves", () => {
    installInvokeRouter({ floatState: new Promise<FloatStateDto>(() => {}) });
    render(<FloatWindow />);
    expect(screen.getByText("正在同步悬浮窗状态…")).toBeInTheDocument();
    // The window must never guess a mode before the backend speaks.
    expect(screen.queryByText("OpenCode Quota Checker")).not.toBeInTheDocument();
  });

  it("renders Full from the backend presentation mode without local state", async () => {
    installInvokeRouter({ usage: usage([windowReport("a", "5 小时", 4), windowReport("b", "每周", 27)]) });
    render(<FloatWindow />);
    expect(await screen.findByText("5 小时")).toBeInTheDocument();
    expect(screen.getByText("每周")).toBeInTheDocument();
    expect(screen.getByText("4.0%")).toBeInTheDocument();
    expect(screen.getByText("27.0%")).toBeInTheDocument();
    expect(screen.getByText("最高 27.0%")).toBeInTheDocument();
    // Full header carries the app title + a compact button.
    expect(screen.getByText("OpenCode Quota Checker")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "切换到精简视图" })).toBeInTheDocument();
  });

  it("switches layout when the float://state event arrives (no stale local mode)", async () => {
    installInvokeRouter({ usage: usage([windowReport("a", "5 小时", 4), windowReport("b", "每周", 27)]) });
    render(<FloatWindow />);
    expect(await screen.findByText("每周")).toBeInTheDocument();

    act(() => {
      tauri.emit(EVENT.FLOAT_STATE, compactState);
    });
    // Compact shows only the highest-risk window and an expand button.
    expect(await screen.findByRole("button", { name: "展开全部配额" })).toBeInTheDocument();
    expect(screen.queryByText("OpenCode Quota Checker")).not.toBeInTheDocument();
    expect(screen.queryByText("4.0%")).not.toBeInTheDocument();
    expect(screen.getByText("27.0%")).toBeInTheDocument();

    act(() => {
      tauri.emit(EVENT.FLOAT_STATE, fullState);
    });
    expect(await screen.findByText("OpenCode Quota Checker")).toBeInTheDocument();
    expect(screen.getByText("4.0%")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "展开全部配额" })).not.toBeInTheDocument();
  });

  it("renders the Docked strip from presentationMode", async () => {
    installInvokeRouter({ floatState: dockedState, usage: usage([windowReport("a", "5 小时", 4)]) });
    render(<FloatWindow />);
    expect(await screen.findByText("展开")).toBeInTheDocument();
    expect(screen.getByText("4.0%")).toBeInTheDocument();
    expect(screen.getByText("剩余 960")).toBeInTheDocument();
    // No Full chrome in the docked strip.
    expect(screen.queryByText("OpenCode Quota Checker")).not.toBeInTheDocument();
    expect(screen.queryByText("最高 4.0%")).not.toBeInTheDocument();
  });

  it("docked Expand restores the configured Full mode", async () => {
    installInvokeRouter({ floatState: dockedState, usage: usage([windowReport("a", "5 小时", 4)]) });
    render(<FloatWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "展开全部配额" }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("set_float_mode", { mode: "full" });
    });
  });
});

describe("FloatWindow views", () => {
  it("renders every quota window in Full mode", async () => {
    installInvokeRouter({
      usage: usage([
        windowReport("a", "5 小时", 4),
        windowReport("b", "每周", 27),
        windowReport("c", "每月", 55),
      ]),
    });
    render(<FloatWindow />);
    expect(await screen.findByText("5 小时")).toBeInTheDocument();
    expect(screen.getByText("每周")).toBeInTheDocument();
    expect(screen.getByText("每月")).toBeInTheDocument();
  });

  it("renders only the highest-risk window in Compact mode", async () => {
    installInvokeRouter({
      floatState: compactState,
      usage: usage([windowReport("a", "5 小时", 4), windowReport("b", "每周", 27)]),
    });
    render(<FloatWindow />);
    expect(await screen.findByText("27.0%")).toBeInTheDocument();
    expect(screen.queryByText("4.0%")).not.toBeInTheDocument();
    expect(screen.getByText("剩余额度")).toBeInTheDocument();
  });

  it("renders long quota labels without breaking the layout", async () => {
    const longLabel = "OpenCode Go 企业版 - 每周配额窗口（自动滚动订阅）".repeat(4);
    installInvokeRouter({ usage: usage([windowReport("a", longLabel, 12)]) });
    render(<FloatWindow />);
    const label = await screen.findByTitle(longLabel);
    expect(label).toBeInTheDocument();
    expect(screen.getByText("12.0%")).toBeInTheDocument();
  });

  it.each([
    [0, "0.0%"],
    [0.1, "0.1%"],
    [50, "50.0%"],
    [99.9, "99.9%"],
    [100, "100.0%"],
  ])("renders the %s%% extreme without overflow", async (percent, text) => {
    installInvokeRouter({
      usage: usage([windowReport("a", "5 小时", percent)]),
    });
    render(<FloatWindow />);
    expect(await screen.findByText(text)).toBeInTheDocument();
  });
});

describe("FloatWindow states", () => {
  it("shows the error state with a working retry", async () => {
    installInvokeRouter({ usage: errorUsage });
    render(<FloatWindow />);
    expect(await screen.findByText("暂时无法更新用量")).toBeInTheDocument();
    expect(screen.getByText(/无法连接 OpenCode 服务/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("refresh_usage");
    });
  });

  it("shows the loading state without a retry button", async () => {
    installInvokeRouter({ usage: loadingUsage });
    render(<FloatWindow />);
    expect(await screen.findByText("正在同步 OpenCode 配额…")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "刷新" })).not.toBeInTheDocument();
  });

  it("shows the empty state with a refresh action", async () => {
    render(<FloatWindow />);
    expect(await screen.findByText("暂无配额数据")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新" })).toBeInTheDocument();
  });
});

describe("FloatWindow controls", () => {
  it("Full → Compact via the header button", async () => {
    installInvokeRouter({ usage: usage([windowReport("a", "5 小时", 4)]) });
    render(<FloatWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "切换到精简视图" }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("set_float_mode", { mode: "compact" });
    });
  });

  it("Compact → Full via the header expand button", async () => {
    installInvokeRouter({
      floatState: compactState,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    render(<FloatWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "展开全部配额" }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("set_float_mode", { mode: "full" });
    });
  });

  it("refresh and close invoke their commands", async () => {
    installInvokeRouter({ usage: usage([windowReport("a", "5 小时", 4)]) });
    render(<FloatWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "立即刷新" }));
    fireEvent.click(screen.getByRole("button", { name: "关闭悬浮窗" }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("refresh_usage");
      expect(tauri.invoke).toHaveBeenCalledWith("close_float_window");
    });
  });

  it("keeps buttons clickable in the docked strip", async () => {
    installInvokeRouter({
      floatState: dockedState,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    render(<FloatWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "关闭悬浮窗" }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("close_float_window");
    });
  });
});
