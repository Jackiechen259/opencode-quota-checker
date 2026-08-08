//! GitHub Releases driven update checks, package download, verification, and
//! platform installation.
//!
//! The updater is a non-critical background feature: every failure path maps
//! to an [`UpdateError`] that is surfaced only in the settings page or a
//! dismissible dashboard banner, and never blocks quota monitoring.

pub mod checker;
pub mod download;
pub mod installer;
pub mod manifest;

pub use checker::{check_for_update, UpdateInfo};
pub use download::{download_task, VerifiedPackage};
pub use installer::{install_update, open_url};

/// Errors produced by the update subsystem.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UpdateError {
    /// The network request for a manifest or package failed.
    #[error("update request failed: {0}")]
    Request(String),
    /// The manifest could not be parsed or is not supported.
    #[error("update manifest is invalid: {0}")]
    Manifest(String),
    /// The asset URL is not one of the trusted release origins.
    #[error("update asset URL is not trusted")]
    UnsafeUrl,
    /// The package checksum did not match the published value.
    #[error("update package integrity verification failed")]
    Verification,
    /// The package could not be downloaded.
    #[error("update package download failed: {0}")]
    Download(String),
    /// The platform installer could not be launched.
    #[error("update install failed: {0}")]
    Install(String),
    /// The current platform has no supported update package.
    #[error("current platform is not supported")]
    UnsupportedPlatform,
}

impl UpdateError {
    /// A concise, user-facing message shown in the settings page.
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Request(_) => "暂时无法检查更新，请稍后重试。",
            Self::Manifest(_) => "更新信息无效，请稍后重试。",
            Self::UnsafeUrl => "更新地址不安全，已停止下载。",
            Self::Verification => "更新包完整性校验失败，请稍后重试。",
            Self::Download(_) => "更新包下载失败，请稍后重试。",
            Self::Install(_) => "无法启动更新安装。",
            Self::UnsupportedPlatform => "当前平台不支持自动更新。",
        }
    }
}
