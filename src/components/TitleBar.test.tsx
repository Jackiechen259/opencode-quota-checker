// TitleBar regression tests: window-control buttons issue exactly one
// window API call each, rejections never crash the component, and the
// controls container is never a drag region.

import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
// Must precede `./TitleBar`: importing this module registers the Tauri mocks.
import { tauri } from "../test/tauri";
import { isInteractiveTarget, TitleBar } from "./TitleBar";

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

  it("double-clicking the spacer toggles maximize", async () => {
    const { container } = render(<TitleBar maximized={false} />);
    const spacer = container.querySelector(".titlebar-spacer") as HTMLElement;
    fireEvent.doubleClick(spacer);
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

  it("drag area primary pointerdown calls startDragging", async () => {
    const { container } = render(<TitleBar maximized={false} />);
    const brand = container.querySelector(".titlebar-brand") as HTMLElement;
    fireEvent.pointerDown(brand, { button: 0 });
    await act(async () => {});
    expect(tauri.windowApi.startDragging).toHaveBeenCalledTimes(1);
  });

  it("button pointerdown does not call startDragging", async () => {
    render(<TitleBar maximized={false} />);
    const button = screen.getByRole("button", { name: "最小化" });
    fireEvent.pointerDown(button, { button: 0 });
    await act(async () => {});
    expect(tauri.windowApi.startDragging).not.toHaveBeenCalled();
  });

  it("non-primary button on drag area does not start dragging", async () => {
    const { container } = render(<TitleBar maximized={false} />);
    const brand = container.querySelector(".titlebar-brand") as HTMLElement;
    fireEvent.pointerDown(brand, { button: 2 });
    await act(async () => {});
    expect(tauri.windowApi.startDragging).not.toHaveBeenCalled();
  });

  it("secondary click on drag area (detail 2) does not start dragging", async () => {
    const { container } = render(<TitleBar maximized={false} />);
    const spacer = container.querySelector(".titlebar-spacer") as HTMLElement;
    // detail 2 simulates the second click of a double-click sequence
    fireEvent.pointerDown(spacer, { button: 0, detail: 2 });
    await act(async () => {});
    expect(tauri.windowApi.startDragging).not.toHaveBeenCalled();
  });

  it("rejected startDragging does not throw unhandled rejection", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    tauri.windowApi.startDragging.mockRejectedValueOnce(new Error("drag failed"));
    const { container } = render(<TitleBar maximized={false} />);
    const brand = container.querySelector(".titlebar-brand") as HTMLElement;
    expect(() => fireEvent.pointerDown(brand, { button: 0 })).not.toThrow();
    await act(async () => {});
    expect(consoleError).toHaveBeenCalledWith(
      "[window] startDragging failed",
      expect.anything(),
    );
    consoleError.mockRestore();
  });

  it("isInteractiveTarget identifies interactive elements", () => {
    const button = document.createElement("button");
    expect(isInteractiveTarget(button)).toBe(true);

    const inner = document.createElement("span");
    button.appendChild(inner);
    // closest walks up to button
    expect(isInteractiveTarget(inner)).toBe(true);

    const div = document.createElement("div");
    expect(isInteractiveTarget(div)).toBe(false);

    expect(isInteractiveTarget(null)).toBe(false);
  });
});
