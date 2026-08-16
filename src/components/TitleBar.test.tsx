// TitleBar regression tests: window-control buttons issue exactly one
// window API call each, rejections never crash the component, and the
// controls container is never a drag region.

import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
// Must precede `./TitleBar`: importing this module registers the Tauri mocks.
import { tauri } from "../test/tauri";
import { TitleBar } from "./TitleBar";

beforeEach(() => {
  tauri.reset();
});

describe("TitleBar", () => {
  it("minimizes through the window API exactly once", async () => {
    render(<TitleBar maximized={false} />);
    fireEvent.click(screen.getByRole("button", { name: "最小化" }));
    await act(async () => {});
    expect(tauri.windowApi.minimize).toHaveBeenCalledTimes(1);
  });

  it("toggles maximize exactly once", async () => {
    render(<TitleBar maximized={false} />);
    fireEvent.click(screen.getByRole("button", { name: "最大化" }));
    await act(async () => {});
    expect(tauri.windowApi.toggleMaximize).toHaveBeenCalledTimes(1);
  });

  it("shows the restore glyph and action when maximized", () => {
    render(<TitleBar maximized={true} />);
    fireEvent.click(screen.getByRole("button", { name: "还原" }));
    expect(tauri.windowApi.toggleMaximize).toHaveBeenCalledTimes(1);
  });

  it("closes through the window API exactly once", async () => {
    render(<TitleBar maximized={false} />);
    fireEvent.click(screen.getByRole("button", { name: "关闭" }));
    await act(async () => {});
    expect(tauri.windowApi.close).toHaveBeenCalledTimes(1);
  });

  it("double-clicking the brand toggles maximize", async () => {
    render(<TitleBar maximized={false} />);
    fireEvent.doubleClick(screen.getByText("OpenCode Quota Checker"));
    await act(async () => {});
    expect(tauri.windowApi.toggleMaximize).toHaveBeenCalledTimes(1);
  });

  it("does not crash when the window API rejects", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    tauri.windowApi.minimize.mockRejectedValueOnce(new Error("permission denied"));
    render(<TitleBar maximized={false} />);
    expect(() => fireEvent.click(screen.getByRole("button", { name: "最小化" }))).not.toThrow();
    await act(async () => {});
    expect(screen.getByRole("button", { name: "最小化" })).toBeInTheDocument();
    expect(consoleError).toHaveBeenCalledWith(
      "[window] minimize failed",
      expect.anything(),
    );
    consoleError.mockRestore();
  });

  it("keeps the controls container out of the drag region", () => {
    const { container } = render(<TitleBar maximized={false} />);
    expect(container.querySelector(".titlebar")).not.toHaveAttribute("data-tauri-drag-region");
    expect(container.querySelector(".titlebar-brand")).toHaveAttribute("data-tauri-drag-region");
    expect(container.querySelector(".titlebar-spacer")).toHaveAttribute("data-tauri-drag-region");
    expect(container.querySelector(".titlebar-controls")).not.toHaveAttribute(
      "data-tauri-drag-region",
    );
    for (const control of container.querySelectorAll(".titlebar-control")) {
      expect(control).not.toHaveAttribute("data-tauri-drag-region");
    }
  });
});
