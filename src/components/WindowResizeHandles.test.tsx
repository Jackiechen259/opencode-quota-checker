import { act, fireEvent, render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { tauri } from "../test/tauri";
import { WindowResizeHandles } from "./WindowResizeHandles";

beforeEach(() => {
  tauri.reset();
});

describe("WindowResizeHandles", () => {
  const directions = [
    ["North", "resize-handle-n"],
    ["South", "resize-handle-s"],
    ["West", "resize-handle-w"],
    ["East", "resize-handle-e"],
    ["NorthWest", "resize-handle-nw"],
    ["NorthEast", "resize-handle-ne"],
    ["SouthWest", "resize-handle-sw"],
    ["SouthEast", "resize-handle-se"],
  ] as const;

  for (const [direction, className] of directions) {
    it(`${direction} handle calls startResizeDragging with ${direction}`, async () => {
      const { container } = render(<WindowResizeHandles maximized={false} />);
      const handle = container.querySelector(`.${className}`) as HTMLElement;
      expect(handle).toBeTruthy();
      fireEvent.pointerDown(handle, { button: 0 });
      await act(async () => {});
      expect(tauri.windowApi.startResizeDragging).toHaveBeenCalledTimes(1);
      expect(tauri.windowApi.startResizeDragging).toHaveBeenCalledWith(direction);
    });
  }

  it("does not render handles when maximized", () => {
    const { container } = render(<WindowResizeHandles maximized={true} />);
    expect(container.querySelectorAll(".resize-handle").length).toBe(0);
  });

  it("renders 8 handles when not maximized", () => {
    const { container } = render(<WindowResizeHandles maximized={false} />);
    expect(container.querySelectorAll(".resize-handle").length).toBe(8);
  });

  it("renders 8 handles when maximized is null (unknown)", () => {
    const { container } = render(<WindowResizeHandles maximized={null} />);
    expect(container.querySelectorAll(".resize-handle").length).toBe(8);
  });

  it("maximized handles do not call startResizeDragging on pointerdown", async () => {
    // When maximized the component returns null, so there is no handle to click.
    // This test documents the contract: no DOM element means no IPC can fire.
    const { container } = render(<WindowResizeHandles maximized={true} />);
    expect(container.innerHTML).toBe("");
    expect(tauri.windowApi.startResizeDragging).not.toHaveBeenCalled();
  });

  it("non-primary button does not trigger resize", async () => {
    const { container } = render(<WindowResizeHandles maximized={false} />);
    const handle = container.querySelector(".resize-handle-n") as HTMLElement;
    fireEvent.pointerDown(handle, { button: 2 });
    await act(async () => {});
    expect(tauri.windowApi.startResizeDragging).not.toHaveBeenCalled();
  });

  it("rejected startResizeDragging does not crash", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    tauri.windowApi.startResizeDragging.mockRejectedValueOnce(new Error("resize denied"));
    const { container } = render(<WindowResizeHandles maximized={false} />);
    const handle = container.querySelector(".resize-handle-se") as HTMLElement;
    expect(() => fireEvent.pointerDown(handle, { button: 0 })).not.toThrow();
    await act(async () => {});
    expect(consoleError).toHaveBeenCalledWith(
      expect.stringContaining("startResizeDragging"),
      expect.anything(),
    );
    consoleError.mockRestore();
  });
});
