// Startup freeze regression tests.
//
// These encode the exact failure that made the app stick on
// "正在检查系统钥匙串…" forever: the frontend used snake_case field names
// for camelCase-serialized Rust DTOs, so `configLoaded` was always
// `undefined` and the boot gate never opened. They also pin the boot state
// machine: rejections and hangs must surface as explicit errors, never as
// an infinite loading screen.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
// Must precede `./App`: importing this module registers the Tauri mocks.
import { tauri } from "./test/tauri";
import { appStatusFixture, bootStatusFixture } from "./test/fixtures/appStatus";
import { MainWindow } from "./App";

const defaultConfig = {
  schema_version: 2,
  monitor_enabled: false,
  monitor_interval_secs: 300,
  thresholds: { five_hour: 80, weekly: 80, monthly: 80 },
  opencode_workspace_id: null,
  close_behavior: "minimize_to_tray",
  float_open: false,
  float_mode: "full",
  float_position: null,
  update_checks_enabled: false,
  auto_download_updates: false,
};

const idleUpdate = {
  status: "idle",
  available: null,
  downloadedVersion: null,
  progress: null,
  error: null,
  lastCheckedMs: null,
  bannerDismissed: false,
};

const noUsage = { report: null, loading: false, error: null, lastSuccessMs: null };

/**
 * Installs an `invoke` router with sane defaults for every command the shell
 * fires at startup, letting individual tests override `get_boot_status` /
 * `get_app_status` only.
 */
function installInvokeRouter(
  overrides: Partial<{
    boot: ReturnType<typeof bootStatusFixture>;
    app: ReturnType<typeof appStatusFixture>;
  }> = {},
) {
  tauri.invoke.mockImplementation(async (command: string) => {
    switch (command) {
      case "get_boot_status":
        return overrides.boot ?? bootStatusFixture();
      case "get_app_status":
        return overrides.app ?? appStatusFixture();
      case "get_usage":
        return noUsage;
      case "get_config":
        return defaultConfig;
      case "get_update_state":
        return idleUpdate;
      default:
        return undefined;
    }
  });
}

const LOADING_TEXT = "正在检查系统钥匙串…";

beforeEach(() => {
  tauri.reset();
  installInvokeRouter();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("MainWindow startup", () => {
  it("opens the credentials page when the camelCase boot status says config is loaded", async () => {
    render(<MainWindow />);
    // The camelCase `configLoaded: true` must open the boot gate; the page
    // must never stick on the loading text.
    expect(await screen.findByText("尚未配置 OpenCode Go")).toBeInTheDocument();
    expect(screen.queryByText(LOADING_TEXT)).not.toBeInTheDocument();
  });

  it("enters the dashboard when credentials are available and configured", async () => {
    installInvokeRouter({
      boot: bootStatusFixture({
        configured: true,
        credentials: { phase: "available", available: true, error: null },
      }),
      app: appStatusFixture({
        configured: true,
        credentials: { phase: "available", available: true, error: null },
      }),
    });
    render(<MainWindow />);
    expect(await screen.findByText("还没有配额数据")).toBeInTheDocument();
    expect(screen.queryByText(LOADING_TEXT)).not.toBeInTheDocument();
  });

  it("shows a recoverable error state for a timed-out keyring, not loading", async () => {
    installInvokeRouter({
      boot: bootStatusFixture({
        credentials: {
          phase: "timeout",
          available: false,
          error: {
            code: "keyring_timeout",
            user: "无法读取系统凭据：系统钥匙串响应超时。请重试。",
            detail: "credential check timed out after 5 seconds",
          },
        },
      }),
    });
    render(<MainWindow />);
    expect(await screen.findByText("重新检查系统钥匙串")).toBeInTheDocument();
    expect(screen.getByText(/系统钥匙串响应超时/)).toBeInTheDocument();
    expect(screen.queryByText(LOADING_TEXT)).not.toBeInTheDocument();
  });

  it("surfaces a get_boot_status rejection as a startup error", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    tauri.invoke.mockImplementation(async (command: string) => {
      if (command === "get_boot_status") throw new Error("ipc exploded");
      switch (command) {
        case "get_app_status":
          return appStatusFixture();
        case "get_usage":
          return noUsage;
        case "get_config":
          return defaultConfig;
        case "get_update_state":
          return idleUpdate;
        default:
          return undefined;
      }
    });
    render(<MainWindow />);
    expect(await screen.findByText("应用初始化失败")).toBeInTheDocument();
    expect(screen.getByText("重新尝试")).toBeInTheDocument();
    expect(screen.queryByText(LOADING_TEXT)).not.toBeInTheDocument();
    // The rejection must be logged, not silently swallowed.
    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith(
        "[startup] get_boot_status failed",
        expect.anything(),
      );
    });
    consoleError.mockRestore();
  });

  it("turns an unanswered get_boot_status into a timeout error, never infinite loading", async () => {
    vi.useFakeTimers();
    tauri.invoke.mockImplementation(async (command: string) => {
      // get_boot_status never settles; the watchdog must break the loading.
      if (command === "get_boot_status") return new Promise(() => {});
      switch (command) {
        case "get_app_status":
          return appStatusFixture();
        case "get_usage":
          return noUsage;
        case "get_config":
          return defaultConfig;
        case "get_update_state":
          return idleUpdate;
        default:
          return undefined;
      }
    });
    render(<MainWindow />);
    expect(screen.getByText(LOADING_TEXT)).toBeInTheDocument();

    await act(async () => {
      vi.advanceTimersByTime(8_001);
    });
    expect(screen.getByText("应用初始化超时")).toBeInTheDocument();
    expect(screen.getByText("重新尝试")).toBeInTheDocument();
    expect(screen.queryByText(LOADING_TEXT)).not.toBeInTheDocument();
  });

  it("recovers when retry succeeds after a boot failure", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    let failing = true;
    tauri.invoke.mockImplementation(async (command: string) => {
      if (command === "get_boot_status") {
        if (failing) throw new Error("transient ipc failure");
        return bootStatusFixture();
      }
      switch (command) {
        case "get_app_status":
          return appStatusFixture();
        case "get_usage":
          return noUsage;
        case "get_config":
          return defaultConfig;
        case "get_update_state":
          return idleUpdate;
        default:
          return undefined;
      }
    });
    render(<MainWindow />);
    expect(await screen.findByText("应用初始化失败")).toBeInTheDocument();

    failing = false;
    fireEvent.click(screen.getByRole("button", { name: "重新尝试" }));
    expect(await screen.findByText("尚未配置 OpenCode Go")).toBeInTheDocument();
    consoleError.mockRestore();
  });

  it("re-renders the body when the app://status event reports a new credential phase", async () => {
    render(<MainWindow />);
    expect(await screen.findByText("尚未配置 OpenCode Go")).toBeInTheDocument();

    // A successful save flow emits the full status with an available phase.
    act(() => {
      tauri.emit("app://status", appStatusFixture({ configured: true }));
    });
    expect(await screen.findByText("还没有配额数据")).toBeInTheDocument();
    expect(screen.queryByText("尚未配置 OpenCode Go")).not.toBeInTheDocument();
  });
});
