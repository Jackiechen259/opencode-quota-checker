// TypeScript mirror of the Rust serde DTOs.
//
// Types from `opencode-core` keep snake_case field names (no `rename_all`);
// the Tauri command DTOs use camelCase (`serde(rename_all = "camelCase")`),
// and every IPC interface below must match the exact serialized JSON keys.
// The Rust integration tests in `src-tauri/tests/state.rs` assert the same
// contract on the Rust side — change both together.

export type FloatMode =
  | "full"
  | "compact"
  | "docked_top"
  | "docked_left"
  | "docked_right";
/** Screen edge a docked floating window is snapped to. */
export type FloatDockEdge = "top" | "left" | "right";
/**
 * Modes that may be persisted to the config. The docked presentations are
 * transient (the window is snapped to a screen edge) and must never be written.
 */
export type PersistedFloatMode = "full" | "compact";
/**
 * What the caption × / Alt+F4 does with the main window.
 *
 * Mirrors the Rust `CloseBehavior` enum; the persisted values are the wire
 * format and must never be renamed.
 */
export type CloseBehavior = "minimize_to_tray" | "minimize_to_taskbar" | "exit";
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
  /** OpenCode has exhausted this window (`status: "rate-limited"`). */
  limited?: boolean;
}

export interface UsageReport {
  plan_type: string;
  windows: WindowReport[];
  fetched_at: number;
}

/**
 * Workspace token usage for one period, from the Console Activity summary.
 *
 * The Rust `TokenUsageReport` has no `rename_all`, so these field names stay
 * snake_case just like `UsageReport`. Counts cross IPC as JSON numbers; the
 * current 30-day totals are far below `Number.MAX_SAFE_INTEGER`.
 */
export interface TokenUsageReport {
  range: string;
  requests: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  cost_microcents: number;
  fetched_at: number;
}

/** IPC snapshot for the Token 用量 card; separate from the quota snapshot. */
export interface TokenUsageDto {
  report: TokenUsageReport | null;
  loading: boolean;
  error: AppError | null;
  lastSuccessMs: number | null;
}

/** One Go allowance meter from `/console/api/go/status`. */
export interface GoMeter {
  used_microcents: number;
  limit_microcents: number;
  resets_at: number;
  percent: number;
  limited: boolean;
}

/** Paid-period access block for the Go subscription. */
export interface GoAccess {
  starts_at: number;
  five_hour: GoMeter | null;
  week: GoMeter | null;
  month: GoMeter | null;
}

/** Go subscription status; accessible only when the key can read usage. */
export interface GoStatusReport {
  product: string;
  cancel_at_period_end: boolean;
  use_balance: boolean;
  access: GoAccess | null;
  fetched_at: number;
}

/** One Go-only daily usage point. */
export interface GoActivityDay {
  date: string;
  cost_microcents: number;
  requests: number;
  tokens: number;
}

/** Per-model request count for the current Go period. */
export interface GoModelUsage {
  provider: string;
  model: string;
  requests: number;
}

/** Go-only activity for the current paid period. */
export interface GoActivityReport {
  starts_at: number;
  daily: GoActivityDay[];
  models: GoModelUsage[];
  total_cost_microcents: number;
  total_requests: number;
  total_tokens: number;
  fetched_at: number;
}

/** Billing credit used by the Extra usage card (optional source). */
export interface BillingReport {
  available_microcents: number;
  balance_microcents: number;
  billing_mode: string;
  fetched_at: number;
}

/** On-demand Go overview snapshot; each source has its own error. */
export interface GoOverviewDto {
  status: GoStatusReport | null;
  activity: GoActivityReport | null;
  billing: BillingReport | null;
  statusError: AppError | null;
  activityError: AppError | null;
  billingError: AppError | null;
  loading: boolean;
  lastSuccessMs: number | null;
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
  float_mode: PersistedFloatMode;
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
 * persisted) and `presentationMode` is what the UI must render right now. A
 * docked presentation already names its edge (`docked_top`, `docked_left`,
 * `docked_right`), so there is no second orientation field that could disagree
 * with the native window size. The frontend renders ONLY from
 * `presentationMode` and never derives it.
 */
export interface FloatStateDto {
  open: boolean;
  configuredMode: PersistedFloatMode;
  presentationMode: FloatMode;
  /**
   * The docked strip is expanded into its hover card.
   *
   * A transient presentation of a docked window (never a fourth layout, never
   * persisted). The backend resizes the native window for it; the view renders
   * the ring card from this flag and never keeps a hover state of its own.
   */
  dockHover: boolean;
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

/**
 * Health bucket for one window.
 *
 * A window OpenCode reports as exhausted is critical no matter what the
 * percentage says: the authoritative `limited` flag wins over the heuristic.
 */
export function quotaHealth(percent: number, limited = false): QuotaHealth {
  if (limited) return "critical";
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
  TOKEN_USAGE_STATE: "token://state",
  GO_OVERVIEW_STATE: "go://overview",
  MONITOR_STATUS: "monitor://status",
  FLOAT_STATE: "float://state",
  UPDATE_STATE: "update://state",
  APP_STATUS: "app://status",
} as const;
