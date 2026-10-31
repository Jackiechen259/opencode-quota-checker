// Floating window regression tests.
//
// The core scenario these pin: the frontend renders EXCLUSIVELY from the
// backend-delivered `presentationMode` and keeps no local mode state, so
// "Full UI inside a Compact native window" cannot happen. Also covered:
// every view, every state (loading/error/empty), extreme percentages, long
// labels, and the window-control commands.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
// Must precede the FloatWindow import: registers the Tauri mocks.
import { tauri } from "../../test/tauri";
import { EVENT, type FloatStateDto, type UsageDto, type WindowReport } from "../../types/models";
import { FloatWindow } from "./FloatWindow";

const fullState: FloatStateDto = {
  open: true,
  configuredMode: "full",
  presentationMode: "full",
  dockHover: false,
};

const compactState: FloatStateDto = {
  open: true,
  configuredMode: "compact",
  presentationMode: "compact",
  dockHover: false,
};

const dockedTopState: FloatStateDto = {
  open: true,
  configuredMode: "full",
  presentationMode: "docked_top",
  dockHover: false,
};

const dockedLeftState: FloatStateDto = {
  open: true,
  configuredMode: "full",
  presentationMode: "docked_left",
  dockHover: false,
};

const dockedRightState: FloatStateDto = {
  open: true,
  configuredMode: "full",
  presentationMode: "docked_right",
  dockHover: false,
};

/** The backend has already grown the window into the hover card. */
const dockedHoverState: FloatStateDto = {
  open: true,
  configuredMode: "full",
  presentationMode: "docked_top",
  dockHover: true,
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
      case "set_float_dock_hover":
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

  it("does not let a late initial snapshot overwrite a newer event", async () => {
    let resolveInitial!: (state: FloatStateDto) => void;
    const lateInitial = new Promise<FloatStateDto>((resolve) => {
      resolveInitial = resolve;
    });
    installInvokeRouter({
      floatState: lateInitial,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    render(<FloatWindow />);
    expect(screen.getByText("正在同步悬浮窗状态…")).toBeInTheDocument();

    act(() => {
      tauri.emit(EVENT.FLOAT_STATE, compactState);
    });
    expect(await screen.findByRole("button", { name: "展开全部配额" })).toBeInTheDocument();

    await act(async () => {
      resolveInitial(fullState);
      await lateInitial;
    });

    // The event is newer than the fetch that was already in flight; the
    // layout must not fall back to Full.
    expect(screen.getByRole("button", { name: "展开全部配额" })).toBeInTheDocument();
    expect(screen.queryByText("OpenCode Quota Checker")).not.toBeInTheDocument();
  });

  it("renders the Docked strip with a mini progress bar and no expand button", async () => {
    installInvokeRouter({ floatState: dockedTopState, usage: usage([windowReport("a", "5 小时", 4)]) });
    render(<FloatWindow />);
    expect(await screen.findByText("5 小时")).toBeInTheDocument();
    expect(screen.getByText("4.0%")).toBeInTheDocument();
    expect(screen.getByText("剩余 960")).toBeInTheDocument();

    const progress = screen.getByRole("progressbar", { name: "5 小时 已用 4.0%" });
    expect(progress).toHaveAttribute("aria-valuenow", "4");
    expect(progress.firstElementChild as HTMLElement).toHaveStyle({ width: "4%" });

    // Docked is transient: dragging un-docks, so no expand control is offered.
    expect(screen.queryByRole("button", { name: "展开全部配额" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "切换到精简视图" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "立即刷新" })).not.toBeInTheDocument();

    // No Full chrome in the docked strip.
    expect(screen.queryByText("OpenCode Quota Checker")).not.toBeInTheDocument();
    expect(screen.queryByText("最高 4.0%")).not.toBeInTheDocument();
  });

  it.each([
    ["top", dockedTopState],
    ["left", dockedLeftState],
    ["right", dockedRightState],
  ])("reports the measured strip length for the %s edge to the backend", async (_edge, state) => {
    const rect = (width: number, height: number) =>
      ({
        width,
        height,
        top: 0,
        left: 0,
        right: width,
        bottom: height,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }) as DOMRect;
    // jsdom has no layout: fake only the out-of-flow clones created by the
    // measurement, and leave everything else at zero. The top strip measures a
    // horizontal `max-content` width, a side rail a vertical `max-content`
    // height.
    const rectSpy = vi
      .spyOn(Element.prototype, "getBoundingClientRect")
      .mockImplementation(function (this: Element) {
        const element = this as HTMLElement;
        if (element.classList.contains("float-docked-content")) {
          if (element.style.width === "max-content") return rect(220, 0);
          if (element.style.height === "max-content") return rect(0, 220);
        }
        return rect(0, 0);
      });
    try {
      installInvokeRouter({
        floatState: state,
        usage: usage([windowReport("a", "5 小时", 4)]),
      });
      render(<FloatWindow />);
      await waitFor(() => {
        // The same length feeds the top strip's width and a side rail's
        // height; the backend decides which axis it applies to.
        expect(tauri.invoke).toHaveBeenCalledWith("set_float_docked_length", {
          length: 268,
        });
      });
    } finally {
      rectSpy.mockRestore();
    }
  });

  it("transposes the progress meter and uses vertical writing in a side rail", async () => {
    installInvokeRouter({
      floatState: dockedLeftState,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    const { container } = render(<FloatWindow />);
    const progress = await screen.findByRole("progressbar", {
      name: "5 小时 已用 4.0%",
    });
    expect(container.querySelector(".float-docked-content")).toHaveClass(
      "float-docked-content--side",
    );
    expect(progress.firstElementChild).toHaveStyle({ width: "100%", height: "4%" });
  });
});

describe("FloatWindow side rails", () => {
  it.each([
    ["left", dockedLeftState],
    ["right", dockedRightState],
  ])("renders the %s rail as a vertical read-only strip", async (edge, state) => {
    installInvokeRouter({
      floatState: state,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    const { container } = render(<FloatWindow />);
    expect(await screen.findByText("5 小时")).toBeInTheDocument();

    const strip = container.querySelector(".float-docked");
    expect(strip).toHaveClass(`float-docked--${edge}`);
    // The whole rail drags, and removing it from the edge expands it.
    expect(strip).toHaveAttribute("data-tauri-drag-region", "deep");

    // Same read-only strip as the top edge: no Full/Compact chrome, and the
    // only control is close.
    expect(screen.queryByText("OpenCode Quota Checker")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "展开全部配额" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "切换到精简视图" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "立即刷新" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "关闭悬浮窗" })).toBeInTheDocument();
  });

  it("switches from a top strip to a side rail on float://state", async () => {
    installInvokeRouter({
      floatState: dockedTopState,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    const { container } = render(<FloatWindow />);
    expect(await screen.findByRole("progressbar", { name: "5 小时 已用 4.0%" })).toBeInTheDocument();
    expect(container.querySelector(".float-docked")).toHaveClass("float-docked--top");

    act(() => {
      tauri.emit(EVENT.FLOAT_STATE, dockedRightState);
    });
    await waitFor(() => {
      expect(container.querySelector(".float-docked")).toHaveClass("float-docked--right");
    });
    expect(container.querySelector(".float-docked")).not.toHaveClass("float-docked--top");
  });
});

describe("FloatWindow docked hover card", () => {
  it("asks the backend to expand only after the pointer rests on the strip", async () => {
    installInvokeRouter({
      floatState: dockedTopState,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    const { container } = render(<FloatWindow />);
    await screen.findByRole("progressbar", { name: "5 小时 已用 4.0%" });
    const root = container.querySelector(".float-window") as HTMLElement;

    fireEvent.pointerOver(root);
    // Hover intent: a pointer passing over the 24 px strip must not flash the
    // card open in the native window.
    expect(tauri.invoke).not.toHaveBeenCalledWith("set_float_dock_hover", { hover: true });

    await waitFor(() =>
      expect(tauri.invoke).toHaveBeenCalledWith("set_float_dock_hover", { hover: true }),
    );
  });

  it("renders one ring per quota window once the backend expands the window", async () => {
    installInvokeRouter({
      floatState: dockedHoverState,
      usage: usage([
        windowReport("monthly", "近一月", 19),
        windowReport("rolling-5h", "5 小时", 78),
        windowReport("weekly", "近一周", 52),
      ]),
    });
    const { container } = render(<FloatWindow />);

    expect(await screen.findByText("配额概览")).toBeInTheDocument();
    // The collapsed strip is replaced by the card, not layered under it.
    expect(container.querySelector(".float-dock-card")).not.toBeNull();
    expect(container.querySelector(".float-docked")).toBeNull();

    // Three rings in the canonical 5h / weekly / monthly order.
    expect(container.querySelectorAll(".float-dock-ring-gauge")).toHaveLength(3);
    expect(screen.getByRole("progressbar", { name: "5 小时 已用 78.0%" })).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "近一周 已用 52.0%" })).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "近一月 已用 19.0%" })).toBeInTheDocument();
    expect(screen.getByText("剩余 220")).toBeInTheDocument();

    // The ring arc is a dash offset of the full circumference, so a 78 % window
    // draws 22 % of it empty.
    const arc = container.querySelectorAll(".float-dock-ring-fill")[0] as Element;
    expect(Number(arc.getAttribute("stroke-dashoffset"))).toBeGreaterThan(0);
    expect(Number(arc.getAttribute("stroke-dasharray"))).toBeGreaterThan(0);

    // The card keeps the strip's only control: closing the window.
    fireEvent.click(screen.getByRole("button", { name: "关闭悬浮窗" }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("close_float_window");
    });
  });

  it("collapses back to the strip when the pointer leaves", async () => {
    installInvokeRouter({
      floatState: dockedHoverState,
      usage: usage([windowReport("rolling-5h", "5 小时", 78)]),
    });
    const { container } = render(<FloatWindow />);
    await screen.findByText("配额概览");

    fireEvent.pointerOut(container.querySelector(".float-window") as HTMLElement);
    await waitFor(() =>
      expect(tauri.invoke).toHaveBeenCalledWith("set_float_dock_hover", { hover: false }),
    );
    // The card stays until the backend confirms it shrank the native window.
    expect(screen.getByText("配额概览")).toBeInTheDocument();

    act(() => {
      tauri.emit(EVENT.FLOAT_STATE, dockedTopState);
    });
    expect(
      await screen.findByRole("progressbar", { name: "5 小时 已用 78.0%" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("配额概览")).not.toBeInTheDocument();
  });

  it("keeps the card open while the pointer is re-entering during the grace period", async () => {
    installInvokeRouter({
      floatState: dockedHoverState,
      usage: usage([windowReport("rolling-5h", "5 小时", 78)]),
    });
    const { container } = render(<FloatWindow />);
    await screen.findByText("配额概览");
    const root = container.querySelector(".float-window") as HTMLElement;

    fireEvent.pointerOut(root);
    fireEvent.pointerOver(root);
    // The re-enter cancelled the pending collapse.
    await new Promise((resolve) => setTimeout(resolve, 320));
    expect(tauri.invoke).not.toHaveBeenCalledWith("set_float_dock_hover", { hover: false });
  });

  it("shows the loading state inside the card when there is no report yet", async () => {
    installInvokeRouter({ floatState: dockedHoverState, usage: loadingUsage });
    render(<FloatWindow />);
    expect(await screen.findByText("配额概览")).toBeInTheDocument();
    expect(screen.getByText("正在同步用量…")).toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });
});

describe("FloatWindow drag regions", () => {
  it("makes the whole header draggable while buttons stay clickable", async () => {
    const { container } = render(<FloatWindow />);
    await screen.findByText("OpenCode Quota Checker");

    expect(container.querySelector(".float-header")).toHaveAttribute(
      "data-tauri-drag-region",
      "deep",
    );
    // A nested bare region would stop Tauri's deep walk before the header.
    expect(container.querySelector(".float-brand")).not.toHaveAttribute(
      "data-tauri-drag-region",
    );
    expect(screen.getByRole("button", { name: "立即刷新" })).not.toHaveAttribute(
      "data-tauri-drag-region",
    );
  });

  it("makes the whole docked strip draggable", async () => {
    installInvokeRouter({
      floatState: dockedTopState,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    const { container } = render(<FloatWindow />);
    await screen.findByRole("progressbar", { name: "5 小时 已用 4.0%" });

    expect(container.querySelector(".float-docked")).toHaveAttribute(
      "data-tauri-drag-region",
      "deep",
    );
    expect(container.querySelector(".float-docked-content")).not.toHaveAttribute(
      "data-tauri-drag-region",
    );
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
      floatState: dockedTopState,
      usage: usage([windowReport("a", "5 小时", 4)]),
    });
    render(<FloatWindow />);
    fireEvent.click(await screen.findByRole("button", { name: "关闭悬浮窗" }));
    await waitFor(() => {
      expect(tauri.invoke).toHaveBeenCalledWith("close_float_window");
    });
  });
});
