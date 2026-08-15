//! Serializable application error that crosses the Tauri IPC boundary.
//!
//! The auth cookie and any other secret is never included in these values:
//! `OpenCodeError` already strips the cookie from HTTP bodies and error
//! messages, and `AppError` only forwards the sanitized `user_message` plus
//! the technical detail string.

use opencode_core::OpenCodeError;
use serde::Serialize;

/// Error payload returned to the frontend by every Tauri command.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    /// Stable machine-readable error kind.
    pub code: String,
    /// Concise message shown in the primary interface.
    pub user: String,
    /// Technical detail shown only in debug/error details.
    pub detail: String,
}

impl AppError {
    /// Wraps a message without leaking internals into the user-facing text.
    pub fn new(
        code: impl Into<String>,
        user: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            user: user.into(),
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} ({})", self.user, self.detail)
    }
}

impl std::error::Error for AppError {}

impl From<OpenCodeError> for AppError {
    fn from(error: OpenCodeError) -> Self {
        let code = match &error {
            OpenCodeError::CredentialsMissing => "credentials_missing",
            OpenCodeError::CredentialsInvalid(_) => "credentials_invalid",
            OpenCodeError::Request(_) => "request_failed",
            OpenCodeError::AuthenticationFailed => "authentication_failed",
            OpenCodeError::WorkspaceNotFound => "workspace_not_found",
            OpenCodeError::RateLimited => "rate_limited",
            OpenCodeError::Parse(_) => "parse_failed",
            OpenCodeError::Http { .. } => "http_error",
            OpenCodeError::Keyring(_) => "keyring_error",
            OpenCodeError::Config(_) => "config_error",
        };
        Self {
            code: code.to_owned(),
            user: error.user_message(),
            detail: error.to_string(),
        }
    }
}

/// Error kind used by the updater subsystem; mirrors the old desktop client.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UpdateError {
    /// The updater plugin rejected the operation (e.g. debug builds).
    #[error("update check failed: {0}")]
    Check(String),
    /// The package could not be downloaded.
    #[error("update package download failed: {0}")]
    Download(String),
    /// The platform installer could not be launched.
    #[error("update install failed: {0}")]
    Install(String),
}

impl UpdateError {
    /// A concise, user-facing message shown in the settings page.
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Check(_) => "暂时无法检查更新，请稍后重试。",
            Self::Download(_) => "更新包下载失败，请稍后重试。",
            Self::Install(_) => "无法启动更新安装。",
        }
    }

    /// Stable machine-readable kind.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Check(_) => "update_check_failed",
            Self::Download(_) => "update_download_failed",
            Self::Install(_) => "update_install_failed",
        }
    }
}

impl From<UpdateError> for AppError {
    fn from(error: UpdateError) -> Self {
        Self {
            code: error.code().to_owned(),
            user: error.user_message().to_owned(),
            detail: error.to_string(),
        }
    }
}
