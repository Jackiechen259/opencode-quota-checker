use reqwest::StatusCode;

/// Errors produced by the OpenCode Quota Checker core.
#[derive(Debug, thiserror::Error)]
pub enum OpenCodeError {
    /// No credential exists in the configured credential store.
    #[error("credentials are not configured")]
    CredentialsMissing,

    /// Credential input is empty or has an invalid persisted representation.
    #[error("invalid credentials: {0}")]
    CredentialsInvalid(String),

    /// The HTTP request could not be completed.
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// The dashboard rejected the supplied authentication.
    #[error("authentication failed")]
    AuthenticationFailed,

    /// The requested workspace does not exist or is not accessible.
    #[error("workspace not found")]
    WorkspaceNotFound,

    /// The dashboard rate-limited the request.
    #[error("request rate limited")]
    RateLimited,

    /// The dashboard response could not be parsed by any known strategy.
    #[error("cannot parse dashboard response: {0}")]
    Parse(String),

    /// The server returned a non-success status.
    #[error("HTTP {status}: {body}")]
    Http {
        /// HTTP response status.
        status: StatusCode,
        /// A bounded response-body excerpt.
        body: String,
    },

    /// The platform credential store failed.
    #[error("credential store error: {0}")]
    Keyring(#[source] keyring::Error),

    /// A non-sensitive configuration value is invalid.
    #[error("configuration error: {0}")]
    Config(String),
}

impl OpenCodeError {
    /// Returns a concise error suitable for the main UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::CredentialsMissing => "尚未配置访问凭证。".to_owned(),
            Self::CredentialsInvalid(_) => "访问凭证无效，请重新输入。".to_owned(),
            Self::Request(error) if error.is_timeout() => "请求超时，请稍后重试。".to_owned(),
            Self::Request(_) => "网络请求失败，请检查网络连接。".to_owned(),
            Self::AuthenticationFailed => {
                "认证已失效，请重新登录 OpenCode Go 并更新 Auth Cookie。".to_owned()
            }
            Self::WorkspaceNotFound => "未找到该工作区，请检查 Workspace ID。".to_owned(),
            Self::RateLimited => "请求过于频繁，请稍后重试。".to_owned(),
            Self::Parse(_) => "OpenCode Go 页面结构已变化，无法解析配额数据。".to_owned(),
            Self::Http { status, .. } => format!("服务请求失败（HTTP {}）。", status.as_u16()),
            Self::Keyring(_) => "无法访问系统钥匙串。".to_owned(),
            Self::Config(message) => format!("配置无效：{message}"),
        }
    }
}
