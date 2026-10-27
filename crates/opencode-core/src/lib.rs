//! Platform-independent domain logic for the OpenCode Quota Checker.

pub mod client;
pub mod credential;
pub mod error;
pub mod models;
pub mod quota;
pub mod threshold;
pub mod usage;

pub use client::OpenCodeClient;
pub use credential::OpenCodeApiKeyStore;
pub use error::OpenCodeError;
pub use models::{UsageReport, WindowReport};
pub use quota::QuotaService;
pub use threshold::{evaluate_alerts, AlertDecision, AlertEvaluation, Thresholds};
