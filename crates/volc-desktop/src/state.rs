use iced::window;
use volc_core::{UsageReport, VolcError};

/// IDs of all windows owned by the application state machine.
#[derive(Debug, Default)]
pub struct WindowState {
    main: Option<window::Id>,
}

impl WindowState {
    /// Returns the current main-window ID.
    pub fn main(&self) -> Option<window::Id> {
        self.main
    }

    /// Records the only main-window instance.
    pub fn set_main(&mut self, id: window::Id) {
        self.main = Some(id);
    }

    /// Removes and returns the main-window ID.
    pub fn take_main(&mut self) -> Option<window::Id> {
        self.main.take()
    }
}

/// Cloneable UI-safe representation of a technical error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiError {
    /// Concise message shown in the primary interface.
    pub user: String,
    /// Technical detail shown only in debug/error details.
    pub detail: String,
}

impl From<VolcError> for UiError {
    fn from(error: VolcError) -> Self {
        Self {
            user: error.user_message(),
            detail: error.to_string(),
        }
    }
}

/// Credential-form and keyring state.
#[derive(Default)]
pub struct CredentialState {
    /// Whether the initial keyring check is running.
    pub checking: bool,
    /// Whether a valid saved credential is available.
    pub configured: bool,
    /// Access Key form value.
    pub access_key: String,
    /// Secret Key form value.
    pub secret_key: String,
    /// Whether a save or clear operation is running.
    pub mutating: bool,
    /// Latest credential-specific error.
    pub error: Option<UiError>,
}

/// Shared usage request and response state.
#[derive(Default)]
pub struct UsageState {
    /// Latest successfully parsed report.
    pub report: Option<UsageReport>,
    /// Latest raw response, held only in memory.
    pub raw: Option<String>,
    /// Whether a parsed report request is in flight.
    pub loading: bool,
    /// Whether a raw request is in flight.
    pub raw_loading: bool,
    /// Latest request error while retaining any previous report.
    pub error: Option<UiError>,
    /// Current display clock used for reset countdowns.
    pub now_ms: i64,
}

impl UsageState {
    /// Creates empty usage state with a current display clock.
    pub fn new() -> Self {
        Self {
            now_ms: chrono::Utc::now().timestamp_millis(),
            ..Self::default()
        }
    }
}
