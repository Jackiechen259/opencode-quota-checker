// TypeScript mirror of the Rust serde DTOs.
//
// Types from `opencode-core` keep snake_case field names (no `rename_all`);
// the Tauri command DTOs use camelCase (`serde(rename_all = "camelCase")`).

export type FloatMode = "full" | "compact" | "docked";
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
  interval_secs: number;
  loading: boolean;
  last_fetch_ms: number | null;
  error: AppError | null;
  notification_error: AppError | null;
}

export interface FloatStateDto {
  open: boolean;
  mode: FloatMode;
  top_docked: boolean;
}

export interface UsageDto {
  report: UsageReport | null;
  loading: boolean;
  error: AppError | null;
  last_success_ms: number | null;
}

export interface UpdateInfoDto {
  version: string;
  tag: string;
  release_notes_url: string;
  body: string | null;
}

export interface UpdateProgressDto {
  downloaded: number;
  total: number | null;
}

export interface UpdateStateDto {
  status: UpdateStatus;
  available: UpdateInfoDto | null;
  downloaded_version: string | null;
  progress: UpdateProgressDto | null;
  error: AppError | null;
  last_checked_ms: number | null;
  banner_dismissed: boolean;
}

export interface AppStatusDto {
  version: string;
  configured: boolean;
  config_loaded: boolean;
  config_error: AppError | null;
  credentials: CredentialStatusDto;
  tray_available: boolean;
  tray_error: string | null;
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
