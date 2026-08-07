//! Platform-independent domain logic for VOLC Status.

pub mod client;
pub mod credential;
pub mod error;
pub mod models;
pub mod opencode;
pub mod signing;
pub mod threshold;

pub use client::ArkClient;
pub use credential::{CredentialStore, Credentials, KeyringCredentialStore, OpenCodeAuthStore};
pub use error::VolcError;
pub use models::{Provider, UsageReport, WindowReport};
pub use threshold::{evaluate_alerts, AlertDecision, AlertEvaluation, Thresholds};
