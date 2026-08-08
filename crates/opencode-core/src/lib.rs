//! Platform-independent domain logic for the OpenCode Quota Checker.

pub mod client;
pub mod credential;
pub mod error;
pub mod models;
pub mod parser;
pub mod quota;
pub mod threshold;

pub use client::OpenCodeClient;
pub use credential::OpenCodeAuthStore;
pub use error::OpenCodeError;
pub use models::{UsageReport, WindowReport};
pub use quota::QuotaService;
pub use threshold::{evaluate_alerts, AlertDecision, AlertEvaluation, Thresholds};
