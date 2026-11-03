//! Platform-independent domain logic for the OpenCode Quota Checker.

pub mod billing;
pub mod client;
pub mod credential;
pub mod error;
pub mod go_usage;
pub mod models;
pub mod quota;
mod serde_util;
pub mod threshold;
pub mod token_usage;
pub mod usage;

pub use billing::{parse_billing_status, BillingReport};
pub use client::OpenCodeClient;
pub use credential::OpenCodeApiKeyStore;
pub use error::OpenCodeError;
pub use go_usage::{
    parse_go_activity, parse_go_status, GoAccess, GoActivityDay, GoActivityReport, GoMeter,
    GoModelUsage, GoStatusReport,
};
pub use models::{UsageReport, WindowReport};
pub use quota::QuotaService;
pub use threshold::{evaluate_alerts, AlertDecision, AlertEvaluation, Thresholds};
pub use token_usage::{parse_token_usage, TokenUsageReport, TOKEN_USAGE_RANGE};
