//! Event names shared between the Rust backend and the React frontend.

/// Fired with a `UsageReport` after every successful quota fetch.
pub const QUOTA_UPDATED: &str = "quota://updated";
/// Fired with an `AppError` after a failed quota fetch.
pub const QUOTA_ERROR: &str = "quota://error";
/// Fired with a `MonitorStatusDto` whenever monitoring state changes.
pub const MONITOR_STATUS: &str = "monitor://status";
/// Fired with a `FloatStateDto` whenever the floating window changes.
pub const FLOAT_STATE: &str = "float://state";
/// Fired with an `UpdateStateDto` whenever the updater changes.
pub const UPDATE_STATE: &str = "update://state";
/// Fired with an `AppStatusDto` after credential/config mutations.
pub const APP_STATUS: &str = "app://status";
