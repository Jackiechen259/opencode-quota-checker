//! Platform-independent domain logic for VOLC Status.

pub mod client;
pub mod credential;
pub mod error;
pub mod models;
pub mod signing;
pub mod threshold;

pub use client::ArkClient;
pub use credential::{CredentialStore, Credentials, KeyringCredentialStore};
pub use error::VolcError;
pub use models::{UsageReport, WindowReport};
pub use threshold::{evaluate_alerts, AlertDecision, AlertEvaluation, Thresholds};
