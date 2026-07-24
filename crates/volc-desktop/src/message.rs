use crate::platform::tray::TrayAction;
use crate::state::UiError;
use iced::window;
use std::fmt;
use volc_core::UsageReport;

/// Input text whose debug representation must never expose its value.
#[derive(Clone)]
pub struct SensitiveInput(pub String);

impl fmt::Debug for SensitiveInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// Every external event and asynchronous result entering the application.
#[derive(Debug, Clone)]
pub enum Message {
    /// The main window finished opening.
    MainWindowOpened(window::Id),
    /// A native close request was received.
    CloseRequested(window::Id),
    /// Poll the tray event bridge.
    PollTray,
    /// A semantic tray action was received.
    Tray(TrayAction),
    /// Close the main window while keeping the daemon alive.
    HideMain,
    /// Initial keyring check completed.
    CredentialsChecked(Result<bool, UiError>),
    /// Access Key input changed.
    AccessKeyChanged(SensitiveInput),
    /// Secret Key input changed.
    SecretKeyChanged(SensitiveInput),
    /// Persist the credential form.
    SaveCredentials,
    /// Credential persistence completed.
    CredentialsSaved(Result<(), UiError>),
    /// Remove the keyring entry.
    ClearCredentials,
    /// Credential removal completed.
    CredentialsCleared(Result<(), UiError>),
    /// Fetch a parsed usage report.
    Refresh,
    /// Parsed usage request completed.
    UsageLoaded(Result<UsageReport, UiError>),
    /// Fetch the unpersisted raw response.
    LoadRaw,
    /// Raw response request completed.
    RawLoaded(Result<String, UiError>),
    /// Stop the daemon and process.
    Exit,
}
