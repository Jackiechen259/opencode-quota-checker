//! OpenCode Go quota integration.
//!
//! OpenCode Go does not currently expose a documented public quota API, so the
//! quota is read from the authenticated workspace dashboard at
//! `https://opencode.ai/workspace/<id>/go`. All OpenCode-specific behavior is
//! isolated behind this module so it can be updated independently when the
//! upstream dashboard changes.

pub mod client;
pub mod parser;
pub mod provider;

pub use client::OpenCodeGoClient;
pub use parser::parse_open_code_go_quota;
pub use provider::OpenCodeGoProvider;
