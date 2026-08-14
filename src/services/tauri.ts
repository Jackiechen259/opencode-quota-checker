// Typed IPC bridge. Every call maps to exactly one Rust `#[tauri::command]`.
// The Rust backend is the single source of truth for quota, config,
// credentials, monitoring, windows, and updates.

import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  AppStatusDto,
  FloatMode,
  FloatStateDto,
  MonitorStatusDto,
  UpdateStateDto,
  UsageDto,
} from "../types/models";

export const api = {
  getAppStatus: () => invoke<AppStatusDto>("get_app_status"),
  quitApp: () => invoke<void>("quit_app"),

  getUsage: () => invoke<UsageDto>("get_usage"),
  refreshUsage: () => invoke<void>("refresh_usage"),
  getRawDashboard: () => invoke<string>("get_raw_dashboard"),

  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<AppConfig>("save_config", { config }),
  setMonitor: (enabled: boolean) => invoke<void>("set_monitor", { enabled }),

  hasCredentials: () => invoke<boolean>("has_credentials"),
  saveCredentials: (workspaceId: string, authCookie: string) =>
    invoke<void>("save_credentials", { workspaceId, authCookie }),
  clearCredentials: () => invoke<void>("clear_credentials"),
  openLoginPage: () => invoke<void>("open_login_page"),

  getMonitorStatus: () => invoke<MonitorStatusDto>("get_monitor_status"),

  getFloatState: () => invoke<FloatStateDto>("get_float_state"),
  openFloatWindow: () => invoke<void>("open_float_window"),
  closeFloatWindow: () => invoke<void>("close_float_window"),
  setFloatMode: (mode: FloatMode) => invoke<void>("set_float_mode", { mode }),

  getUpdateState: () => invoke<UpdateStateDto>("get_update_state"),
  checkForUpdate: () => invoke<void>("check_for_update"),
  downloadUpdate: () => invoke<void>("download_update"),
  installUpdate: () => invoke<void>("install_update"),
  dismissUpdate: () => invoke<void>("dismiss_update"),
  openReleaseNotes: () => invoke<void>("open_release_notes"),
};
