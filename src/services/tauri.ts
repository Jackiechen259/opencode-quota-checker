// Typed IPC bridge. Every call maps to exactly one Rust `#[tauri::command]`.
// The Rust backend is the single source of truth for quota, config,
// credentials, monitoring, windows, and updates.

import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  AppStatusDto,
  BootStatusDto,
  FloatStateDto,
  GoOverviewDto,
  MonitorStatusDto,
  PersistedFloatMode,
  TokenUsageDto,
  UpdateStateDto,
  UsageDto,
} from "../types/models";

export const api = {
  /** Boot-critical status; the only command the startup screen waits on. */
  getBootStatus: () => invoke<BootStatusDto>("get_boot_status"),
  getAppStatus: () => invoke<AppStatusDto>("get_app_status"),
  quitApp: () => invoke<void>("quit_app"),

  getUsage: () => invoke<UsageDto>("get_usage"),
  getTokenUsage: () => invoke<TokenUsageDto>("get_token_usage"),
  refreshUsage: () => invoke<void>("refresh_usage"),
  getRawUsage: () => invoke<string>("get_raw_usage"),

  getGoOverview: () => invoke<GoOverviewDto>("get_go_overview"),
  refreshGoOverview: () => invoke<GoOverviewDto>("refresh_go_overview"),
  openGoPage: () => invoke<void>("open_go_page"),

  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<AppConfig>("save_config", { config }),
  setMonitor: (enabled: boolean) => invoke<void>("set_monitor", { enabled }),

  hasCredentials: () => invoke<boolean>("has_credentials"),
  recheckCredentials: () => invoke<void>("recheck_credentials"),
  saveCredentials: (workspaceId: string | null, apiKey: string) =>
    invoke<void>("save_credentials", { workspaceId, apiKey }),
  testConnection: () => invoke<void>("test_connection"),
  clearCredentials: () => invoke<void>("clear_credentials"),
  openConsolePage: () => invoke<void>("open_console_page"),

  getMonitorStatus: () => invoke<MonitorStatusDto>("get_monitor_status"),

  getFloatState: () => invoke<FloatStateDto>("get_float_state"),
  openFloatWindow: () => invoke<void>("open_float_window"),
  closeFloatWindow: () => invoke<void>("close_float_window"),
  setFloatMode: (mode: PersistedFloatMode) => invoke<void>("set_float_mode", { mode }),
  setFloatDockedLength: (length: number) =>
    invoke<void>("set_float_docked_length", { length }),
  /** Asks the backend to expand/collapse the docked strip's hover card. */
  setFloatDockHover: (hover: boolean) => invoke<void>("set_float_dock_hover", { hover }),

  getUpdateState: () => invoke<UpdateStateDto>("get_update_state"),
  checkForUpdate: () => invoke<void>("check_for_update"),
  downloadUpdate: () => invoke<void>("download_update"),
  installUpdate: () => invoke<void>("install_update"),
  dismissUpdate: () => invoke<void>("dismiss_update"),
  openReleaseNotes: () => invoke<void>("open_release_notes"),
};
