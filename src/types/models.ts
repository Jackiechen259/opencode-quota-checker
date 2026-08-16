// TypeScript mirror of the Rust serde DTOs.
//
// Types from `opencode-core` keep snake_case field names (no `rename_all`);
// the Tauri command DTOs use camelCase (`serde(rename_all = "camelCase")`),
// and every IPC interface below must match the exact serialized JSON keys.
// The Rust integration tests in `src-tauri/tests/state.rs` assert the same
// contract on the Rust side — change both together.

export type FloatMode = "full" | "compact" | "docked";
/**
 * Modes that may be persisted to the config. `docked` is a transient
 * presentation (snapped to the monitor top) and must never be written.
 */
export type PersistedFloatMode = Exclude<FloatMode, "docked">;
export type CloseBehavior = "minimize_to_tray" | "exit";
export type CredentialPhase =
  | "checking"
  | "available"
  | "missing"
  | "error"
  | "timeout";
export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready_to_install"
  | "installing"
  | "up_to_date"
  | "error";

export interface AppError {
  code: string;
  user: string;
  detail: string;
}

export interface WindowReport {
  key: string;
  label: string;
  quota: number;
  used: number;
  remaining: number;
  percent: number;
  subscribe_time: number;
  reset_time: number;
  reset_in_secs: number;
}

export interface UsageReport {
  plan_type: string;
  windows: WindowReport[];
  fetched_at: number;
}

export interface Thresholds {
  five_hour: number;
  weekly: number;
  monthly: number;
}

export interface FloatPosition {
  x: number;
  y: number;
}

export interface AppConfig {
  schema_version: number;
  monitor_enabled: boolean;
  monitor_interval_secs: number;
  thresholds: Thresholds;
  opencode_workspace_id: string | null;
  close_behavior: CloseBehavior;
  float_open: boolean;
  float_mode: FloatMode;
  float_position: FloatPosition | null;
  update_checks_enabled: boolean;
  auto_download_updates: boolean;
}

export interface CredentialStatusDto {
  phase: CredentialPhase;
  available: boolean;
  error: AppError | null;
}

export interface MonitorStatusDto {
  enabled: boolean;
  intervalSecs: number;
  loading: boolean;
  lastFetchMs: number | null;
  error: AppError | null;
  notificationError: AppError | null;
}

/**
 * Floating-window snapshot crossing the IPC boundary.
 *
 * `configuredMode` is the persisted layout (full | compact — docked is never
 * persisted) and `presentationMode` is what the UI must render right now
 * (docked while the window is snapped to the monitor top, otherwise the
 * configured mode). The frontend renders ONLY from `presentationMode` and
 * never derives it, so the native window size and the React UI always agree.
 */
export interface FloatStateDto {
  open: boolean;
  configuredMode: PersistedFloatMode;
  presentationMode: FloatMode;
  topDocked: boolean;
}

export interface UsageDto {
  report: UsageReport | null;
  loading: boolean;
  error: AppError | null;
  lastSuccessMs: number | null;
}

export interface UpdateInfoDto {
  version: string;
  tag: string;
  releaseNotesUrl: string;
  body: string | null;
}

export interface UpdateProgressDto {
  downloaded: number;
  total: number | null;
}

export interface UpdateStateDto {
  status: UpdateStatus;
  available: UpdateInfoDto | null;
  downloadedVersion: string | null;
  progress: UpdateProgressDto | null;
  error: AppError | null;
  lastCheckedMs: number | null;
  bannerDismissed: boolean;
}

/** Boot-critical subset of the app status, from `get_boot_status`. */
export interface BootStatusDto {
  version: string;
  configured: boolean;
  configLoaded: boolean;
  configError: AppError | null;
  credentials: CredentialStatusDto;
}

export interface AppStatusDto {
  version: string;
  configured: boolean;
  configLoaded: boolean;
  configError: AppError | null;
  credentials: CredentialStatusDto;
  trayAvailable: boolean;
  trayError: string | null;
  monitor: MonitorStatusDto;
  float: FloatStateDto;
  update: UpdateStateDto;
}

/** Health bucket used by the UI, matching the archived Iced `QuotaHealth`. */
export type QuotaHealth = "healthy" | "warning" | "critical";

export function quotaHealth(percent: number): QuotaHealth {
  if (percent >= 90) return "critical";
  if (percent >= 70) return "warning";
  return "healthy";
}

export function healthLabel(health: QuotaHealth): string {
  switch (health) {
    case "healthy":
      return "健康";
    case "warning":
      return "接近阈值";
    case "critical":
      return "危险";
  }
}

export function healthStatusLabel(health: QuotaHealth): string {
  switch (health) {
    case "healthy":
      return "正常";
    case "warning":
      return "警告";
    case "critical":
      return "危险";
  }
}

export const EVENT = {
  QUOTA_UPDATED: "quota://updated",
  QUOTA_ERROR: "quota://error",
  MONITOR_STATUS: "monitor://status",
  FLOAT_STATE: "float://state",
  UPDATE_STATE: "update://state",
  APP_STATUS: "app://status",
} as const;
