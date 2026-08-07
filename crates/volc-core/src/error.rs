use reqwest::StatusCode;

/// Errors produced by the VOLC Status core.
#[derive(Debug, thiserror::Error)]
pub enum VolcError {
    /// No credential exists in the configured credential store.
    #[error("credentials are not configured")]
    CredentialsMissing,

    /// Credential input is empty or has an invalid persisted representation.
    #[error("invalid credentials: {0}")]
    CredentialsInvalid(String),

    /// The HTTP request could not be completed.
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// The server returned a non-success status.
    #[error("HTTP {status}: {body}")]
    Http {
        /// HTTP response status.
        status: StatusCode,
        /// A bounded response-body excerpt.
        body: String,
    },

    /// The API returned a structured error.
    #[error("API returned an error [{code}]: {message}")]
    Api {
        /// Provider error code.
        code: String,
        /// Provider error message.
        message: String,
    },

    /// The response could not be decoded.
    #[error("invalid response: {0}")]
    Response(#[from] serde_json::Error),

    /// A decoded response violates the domain model.
    #[error("invalid response: {0}")]
    ResponseValue(String),

    /// The signing input or cryptographic operation is invalid.
    #[error("request signing failed: {0}")]
    Signing(String),

    /// The platform credential store failed.
    #[error("credential store error: {0}")]
    Keyring(#[source] keyring::Error),

    /// A non-sensitive configuration value is invalid.
    #[error("configuration error: {0}")]
    Config(String),
}

impl VolcError {
    /// Returns a concise error suitable for the main UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::CredentialsMissing => "尚未配置访问凭证。".to_owned(),
            Self::CredentialsInvalid(_) => "访问凭证无效，请重新输入。".to_owned(),
            Self::Request(error) if error.is_timeout() => "请求超时，请稍后重试。".to_owned(),
            Self::Request(_) => "网络请求失败，请检查网络连接。".to_owned(),
            Self::Http { status, .. } => format!("服务请求失败（HTTP {}）。", status.as_u16()),
            Self::Api { code, message } => format!("方舟接口错误 [{code}]：{message}"),
            Self::Response(_) | Self::ResponseValue(_) => "接口响应格式无法识别。".to_owned(),
            Self::Signing(_) => "请求签名失败。".to_owned(),
            Self::Keyring(_) => "无法访问系统钥匙串。".to_owned(),
            Self::Config(message) => format!("配置无效：{message}"),
        }
    }
}
