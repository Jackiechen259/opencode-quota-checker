// Settings button-behavior regression tests.
//
// They pin the parts of the settings UX that are easy to break: one config
// write per action, no stale snapshots overwriting backend fields, stop
// working with an invalid draft, accurate pending labels, and no swallowed
// IPC rejections.

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
// Must precede `./Settings`: importing this module registers the Tauri mocks.
import { tauri } from "../test/tauri";
import { appStatusFixture } from "../test/fixtures/appStatus";
import type { AppConfig, AppStatusDto, UpdateStateDto } from "../types/models";
import { Settings } from "./Settings";

function configFixture(overrides: Partial<AppConfig> = {}): AppConfig {
  return {
    schema_version: 2,
    monitor_enabled: false,
    monitor_interval_secs: 300,
    thresholds: { five_hour: 80, weekly: 85, monthly: 90 },
    opencode_workspace_id: null,
    close_behavior: "minimize_to_tray",
    float_open: false,
    float_mode: "full",
    float_position: null,
    update_checks_enabled: true,
    auto_download_updates: true,
    ...overrides,
  };
}

function updateFixture(overrides: Partial<UpdateStateDto> = {}): UpdateStateDto {
  return {
    status: "idle",
    available: null,
    downloadedVersion: null,
    progress: null,
    error: null,
    lastCheckedMs: null,
    bannerDismissed: false,
    ...overrides,
  };
}

type RouterOverrides = {
  config?: AppConfig;
  saveCredentials?: (workspaceId: string | null, apiKey: string) => Promise<void>;
  clearCredentials?: () => Promise<void>;
  testConnection?: () => Promise<void>;
  checkForUpdate?: () => Promise<void>;
  installUpdate?: () => Promise<void>;
  openConsolePage?: () => Promise<void>;
};

let storedConfig: AppConfig;
let commandCalls: string[];
let saveConfigCalls: AppConfig[];
let saveCredentialsCalls: Array<{ workspaceId: string | null; apiKey: string }>;

/**
 * Installs an invoke router that behaves like the Rust backend for the
 * commands used by the settings page: `save_config` returns and stores the
 * canonical config, and `save_credentials` persists the workspace id.
 */
function installRouter(overrides: RouterOverrides = {}) {
  storedConfig = overrides.config ?? configFixture();
  commandCalls = [];
  saveConfigCalls = [];
  saveCredentialsCalls = [];

  tauri.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    commandCalls.push(command);
    switch (command) {
      case "get_config":
        return storedConfig;
      case "save_config": {
        const next = (args as { config: AppConfig }).config;
        saveConfigCalls.push(next);
        storedConfig = next;
        return storedConfig;
      }
      case "save_credentials": {
        const { workspaceId, apiKey } = args as {
          workspaceId: string | null;
          apiKey: string;
        };
        saveCredentialsCalls.push({ workspaceId, apiKey });
        if (overrides.saveCredentials) {
          await overrides.saveCredentials(workspaceId, apiKey);
        } else {
          storedConfig = { ...storedConfig, opencode_workspace_id: workspaceId };
        }
        return undefined;
      }
      case "clear_credentials":
        if (overrides.clearCredentials) {
          await overrides.clearCredentials();
        } else {
          storedConfig = {
            ...storedConfig,
            opencode_workspace_id: null,
            monitor_enabled: false,
          };
        }
        return undefined;
      case "test_connection":
        await overrides.testConnection?.();
        return undefined;
      case "check_for_update":
        await overrides.checkForUpdate?.();
        return undefined;
      case "install_update":
        await overrides.installUpdate?.();
        return undefined;
      case "open_console_page":
        await overrides.openConsolePage?.();
        return undefined;
      default:
        return undefined;
    }
  });
}

function renderSettings(options: {
  config?: AppConfig;
  status?: AppStatusDto;
  update?: UpdateStateDto | null;
} = {}) {
  const config = options.config ?? storedConfig;
  const status =
    options.status ??
    appStatusFixture({
      configured: true,
      credentials: { phase: "available", available: true, error: null },
    });
  const update = options.update ?? updateFixture();
  const onConfigSaved = vi.fn();
  const onRefreshStatus = vi.fn();
  const onClose = vi.fn();

  render(
    <Settings
      status={status}
      config={config}
      update={update}
      onClose={onClose}
      onConfigSaved={onConfigSaved}
      onRefreshStatus={onRefreshStatus}
    />,
  );

  return { onConfigSaved, onRefreshStatus, onClose };
}

beforeEach(() => {
  tauri.reset();
  installRouter();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Settings monitor buttons", () => {
  it("starts monitoring with one save_config call and no redundant set_monitor", async () => {
    installRouter({ config: configFixture({ monitor_enabled: false }) });
    const { onConfigSaved } = renderSettings({ config: storedConfig });

    fireEvent.click(screen.getByRole("button", { name: "启动监控" }));

    await waitFor(() => expect(saveConfigCalls).toHaveLength(1));
    expect(commandCalls).toContain("get_config");
    expect(commandCalls).not.toContain("set_monitor");
    expect(saveConfigCalls[0]).toMatchObject({
      monitor_enabled: true,
      monitor_interval_secs: 300,
    });
    expect(onConfigSaved).toHaveBeenCalledWith(
      expect.objectContaining({ monitor_enabled: true }),
    );
    await waitFor(() => expect(commandCalls).toContain("refresh_usage"));
    expect(await screen.findByText("监控已启动。")).toBeInTheDocument();
  });

  it("applies draft interval changes while keeping monitoring enabled", async () => {
    installRouter({
      config: configFixture({ monitor_enabled: true, monitor_interval_secs: 300 }),
    });
    renderSettings({ config: storedConfig });

    const saveButton = screen.getByRole("button", { name: "保存更改" });
    expect(saveButton).toBeDisabled();

    fireEvent.change(screen.getByLabelText("检查间隔（秒）"), {
      target: { value: "600" },
    });
    expect(saveButton).toBeEnabled();
    fireEvent.click(saveButton);

    await waitFor(() => expect(saveConfigCalls).toHaveLength(1));
    expect(saveConfigCalls[0]).toMatchObject({
      monitor_enabled: true,
      monitor_interval_secs: 600,
    });
    expect(await screen.findByText("监控设置已保存。")).toBeInTheDocument();
  });

  it("rejects an invalid draft for apply but can still stop the monitor", async () => {
    installRouter({
      config: configFixture({ monitor_enabled: true, monitor_interval_secs: 300 }),
    });
    renderSettings({ config: storedConfig });

    fireEvent.change(screen.getByLabelText("检查间隔（秒）"), {
      target: { value: "5" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存更改" }));

    expect(
      await screen.findByText("检查间隔必须是 30–3600 之间的整数。"),
    ).toBeInTheDocument();
    expect(saveConfigCalls).toHaveLength(0);

    fireEvent.click(screen.getByRole("button", { name: "停止监控" }));

    await waitFor(() => expect(saveConfigCalls).toHaveLength(1));
    // Stop is persisted from the canonical config; the invalid draft ("5")
    // must never be written just to stop monitoring.
    expect(saveConfigCalls[0]).toMatchObject({
      monitor_enabled: false,
      monitor_interval_secs: 300,
    });
    expect(await screen.findByText("监控已停止。")).toBeInTheDocument();
  });
});

describe("Settings connection buttons", () => {
  it("saves a workspace-only change without asking for the API key again", async () => {
    installRouter({ config: configFixture({ opencode_workspace_id: null }) });
    renderSettings({ config: storedConfig });

    const saveButton = screen.getByRole("button", { name: "保存 Workspace ID" });
    expect(saveButton).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Workspace ID"), {
      target: { value: "  ws_override  " },
    });
    expect(saveButton).toBeEnabled();
    fireEvent.click(saveButton);

    await waitFor(() => expect(saveConfigCalls).toHaveLength(1));
    expect(saveConfigCalls[0].opencode_workspace_id).toBe("ws_override");
    expect(saveCredentialsCalls).toHaveLength(0);
    expect(await screen.findByText("Workspace ID 已保存。")).toBeInTheDocument();
  });

  it("adopts the backend workspace after saving credentials so later saves keep it", async () => {
    installRouter({
      config: configFixture({ opencode_workspace_id: null, auto_download_updates: true }),
    });
    const { onConfigSaved, onRefreshStatus } = renderSettings({ config: storedConfig });

    fireEvent.change(screen.getByLabelText("Workspace ID"), {
      target: { value: "ws_live" },
    });
    fireEvent.change(screen.getByLabelText("API Key"), {
      target: { value: "oc_sk_test_key" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存连接" }));

    await waitFor(() => expect(saveCredentialsCalls).toHaveLength(1));
    expect(saveCredentialsCalls[0]).toEqual({
      workspaceId: "ws_live",
      apiKey: "oc_sk_test_key",
    });
    await waitFor(() =>
      expect(onConfigSaved).toHaveBeenCalledWith(
        expect.objectContaining({ opencode_workspace_id: "ws_live" }),
      ),
    );
    expect(onRefreshStatus).toHaveBeenCalled();
    expect((screen.getByLabelText("API Key") as HTMLInputElement).value).toBe("");
    expect(
      await screen.findByText("连接已校验并保存到系统钥匙串。"),
    ).toBeInTheDocument();

    // The component still holds the old config snapshot (null workspace), but
    // the next save must start from the backend's latest config.
    fireEvent.click(screen.getByRole("switch", { name: "自动下载更新" }));
    await waitFor(() => expect(saveConfigCalls).toHaveLength(1));
    expect(saveConfigCalls[0].opencode_workspace_id).toBe("ws_live");
  });

  it("labels only the pending connection action and ignores a second click", async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    installRouter({
      config: configFixture(),
      saveCredentials: async () => {
        await gate;
      },
    });
    renderSettings({ config: storedConfig });

    fireEvent.change(screen.getByLabelText("API Key"), {
      target: { value: "oc_sk_slow" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存连接" }));

    const pending = await screen.findByRole("button", { name: "保存中…" });
    expect(pending).toBeDisabled();
    expect(screen.getByRole("button", { name: "测试连接" })).toBeDisabled();
    fireEvent.click(pending);
    expect(saveCredentialsCalls).toHaveLength(1);

    release();
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "保存中…" })).not.toBeInTheDocument(),
    );
    expect(await screen.findByText("连接已校验并保存到系统钥匙串。")).toBeInTheDocument();
  });

  it("disables credential-dependent actions when no key is stored", () => {
    installRouter({ config: configFixture({ monitor_enabled: false }) });
    renderSettings({
      config: storedConfig,
      status: appStatusFixture({
        credentials: { phase: "missing", available: false, error: null },
      }),
    });

    expect(screen.getByRole("button", { name: "启动监控" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "测试连接" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "清除凭证" })).toBeDisabled();
    expect(screen.getByText("保存 API Key 后才能启动监控")).toBeInTheDocument();
  });

  it("clears credentials only after confirmation and adopts the cleared config", async () => {
    installRouter({
      config: configFixture({ monitor_enabled: true, opencode_workspace_id: "ws_old" }),
    });
    const { onConfigSaved, onRefreshStatus } = renderSettings({ config: storedConfig });

    fireEvent.click(screen.getByRole("button", { name: "清除凭证" }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(commandCalls).not.toContain("clear_credentials");

    fireEvent.click(screen.getByRole("button", { name: "确认清除" }));

    await waitFor(() => expect(commandCalls).toContain("clear_credentials"));
    await waitFor(() =>
      expect(onConfigSaved).toHaveBeenCalledWith(
        expect.objectContaining({
          monitor_enabled: false,
          opencode_workspace_id: null,
        }),
      ),
    );
    expect(onRefreshStatus).toHaveBeenCalled();
    expect(await screen.findByText("连接已清除，监控已停止。")).toBeInTheDocument();
  });
});

describe("Settings close behavior", () => {
  it("saves the chosen close behavior through one canonical config write", async () => {
    installRouter({ config: configFixture({ close_behavior: "minimize_to_tray" }) });
    const { onConfigSaved } = renderSettings({ config: storedConfig });

    expect(screen.getByRole("radio", { name: "隐藏到系统托盘" })).toBeChecked();

    fireEvent.click(screen.getByRole("radio", { name: "最小化到任务栏" }));

    await waitFor(() => expect(saveConfigCalls).toHaveLength(1));
    expect(saveConfigCalls[0]).toMatchObject({
      close_behavior: "minimize_to_taskbar",
      monitor_interval_secs: 300,
      auto_download_updates: true,
    });
    expect(onConfigSaved).toHaveBeenCalledWith(
      expect.objectContaining({ close_behavior: "minimize_to_taskbar" }),
    );
    expect(await screen.findByText("关闭行为已保存。")).toBeInTheDocument();
  });

  it("keeps the tray option available while the tray works", () => {
    installRouter({ config: configFixture() });
    renderSettings({ config: storedConfig });

    expect(screen.getByRole("radio", { name: "隐藏到系统托盘" })).toBeEnabled();
    expect(screen.getByRole("radio", { name: "退出应用" })).toBeEnabled();
  });

  it("disables the tray option when no tray icon exists", () => {
    installRouter({ config: configFixture() });
    renderSettings({
      config: storedConfig,
      status: appStatusFixture({
        configured: true,
        credentials: { phase: "available", available: true, error: null },
        trayAvailable: false,
        trayError: "no appindicator",
      }),
    });

    // Hiding without a tray icon would strand the process, so the option is
    // blocked and the fallback (exit) is explained in the field hint.
    expect(screen.getByRole("radio", { name: "隐藏到系统托盘" })).toBeDisabled();
    expect(screen.getByRole("radio", { name: "最小化到任务栏" })).toBeEnabled();
    expect(
      screen.getByText(/「隐藏到系统托盘」会退回到「退出应用」/),
    ).toBeInTheDocument();
  });
});

describe("Settings update buttons", () => {
  it("shows the real update state and keeps auto-download independent of auto-check", async () => {
    installRouter({
      config: configFixture({ update_checks_enabled: false, auto_download_updates: true }),
    });
    renderSettings({
      config: storedConfig,
      update: updateFixture({
        status: "available",
        available: {
          version: "0.3.0",
          tag: "v0.3.0",
          releaseNotesUrl: "https://example.test/v0.3.0",
          body: null,
        },
      }),
    });

    expect(screen.getByText("有可用更新")).toBeInTheDocument();
    expect(screen.queryByText("未检查")).not.toBeInTheDocument();

    // Manual checks honor `auto_download_updates`, so the toggle must stay
    // usable even when scheduled checks are off.
    expect(
      screen.getByText("手动检查发现新版本时也会自动下载"),
    ).toBeInTheDocument();
    const autoDownload = screen.getByRole("switch", { name: "自动下载更新" });
    expect(autoDownload).toBeEnabled();
    fireEvent.click(autoDownload);
    await waitFor(() => expect(saveConfigCalls).toHaveLength(1));
    expect(saveConfigCalls[0]).toMatchObject({
      update_checks_enabled: false,
      auto_download_updates: false,
    });
  });

  it("surfaces a rejected manual update check instead of swallowing it", async () => {
    installRouter({
      config: configFixture(),
      checkForUpdate: async () => {
        throw {
          code: "update_check_failed",
          user: "暂时无法检查更新，请稍后重试。",
          detail: "debug build",
        };
      },
    });
    renderSettings({ config: storedConfig });

    fireEvent.click(screen.getByRole("button", { name: "检查更新" }));

    expect(
      await screen.findByText("暂时无法检查更新，请稍后重试。"),
    ).toBeInTheDocument();
  });

  it("requires confirmation before installing a ready update", async () => {
    installRouter({ config: configFixture() });
    renderSettings({
      config: storedConfig,
      update: updateFixture({ status: "ready_to_install", downloadedVersion: "0.3.0" }),
    });

    fireEvent.click(screen.getByRole("button", { name: "安装并重启" }));
    const dialog = screen.getByRole("dialog");
    expect(commandCalls).not.toContain("install_update");

    fireEvent.click(within(dialog).getByRole("button", { name: "安装并重启" }));

    await waitFor(() => expect(commandCalls).toContain("install_update"));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
