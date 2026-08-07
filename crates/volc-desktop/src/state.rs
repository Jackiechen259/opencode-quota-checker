use crate::message::HeaderAction;
use iced::window;
use std::collections::HashMap;
use volc_core::{UsageReport, VolcError};
/// IDs of all windows owned by the application state machine.
#[derive(Debug, Default)]
pub struct WindowState {
    main: Option<window::Id>,
    floating: Option<window::Id>,
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

    /// Returns the current floating-window ID.
    pub fn floating(&self) -> Option<window::Id> {
        self.floating
    }

    /// Records the only floating-window instance.
    pub fn set_floating(&mut self, id: window::Id) {
        self.floating = Some(id);
    }

    /// Removes and returns the floating-window ID.
    pub fn take_floating(&mut self) -> Option<window::Id> {
        self.floating.take()
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

/// Per-provider credential availability reported by the boot keyring check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderAvailability {
    /// Whether a valid Volcano Ark AK/SK pair is stored.
    pub ark: bool,
    /// Whether an OpenCode Go auth cookie is stored.
    pub opencode: bool,
}

/// Credential-form and keyring state.
#[derive(Default)]
pub struct CredentialState {
    /// Whether the initial keyring check is running.
    pub checking: bool,
    /// Whether a valid Volcano Ark credential is available.
    pub ark: bool,
    /// Whether an OpenCode Go auth cookie is available.
    pub opencode: bool,
    /// Access Key form value.
    pub access_key: String,
    /// Secret Key form value.
    pub secret_key: String,
    /// OpenCode Go Workspace ID form value.
    pub opencode_workspace: String,
    /// OpenCode Go auth cookie form value.
    pub opencode_cookie: String,
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

/// Editable settings overlay state.
pub struct SettingsState {
    /// Whether the settings overlay is visible.
    pub open: bool,
    /// Polling interval input.
    pub interval: String,
    /// Five-hour threshold input.
    pub five_hour: String,
    /// Weekly threshold input.
    pub weekly: String,
    /// Monthly threshold input.
    pub monthly: String,
    /// Whether a config write is in progress.
    pub saving: bool,
    /// Latest validation or write error.
    pub error: Option<UiError>,
    /// Latest successful operation message.
    pub notice: Option<String>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            open: false,
            interval: "300".to_owned(),
            five_hour: "80".to_owned(),
            weekly: "85".to_owned(),
            monthly: "85".to_owned(),
            saving: false,
            error: None,
            notice: None,
        }
    }
}

/// Monitoring notification and deduplication state.
#[derive(Default)]
pub struct MonitorState {
    /// Last alerted subscription cycle per quota window.
    pub last_alerted: HashMap<String, i64>,
    /// Latest notification delivery error.
    pub notification_error: Option<UiError>,
}

/// Transient floating-window persistence state.
#[derive(Default)]
pub struct FloatState {
    /// Whether a move event is waiting for debounced persistence.
    pub position_dirty: bool,
    /// Whether the floating window is temporarily snapped to the monitor top.
    pub top_docked: bool,
}

/// Transient overlays and user feedback.
#[derive(Default)]
pub struct UiState {
    /// Whether the raw-response overlay is visible.
    pub debug_open: bool,
    /// Short user feedback shown without blocking interaction.
    pub toast: Option<String>,
    /// Whether the delete-credential confirmation modal is open.
    pub confirm_clear_credentials: bool,
    /// Keyboard focus used by the compact header action group.
    pub header_focus: Option<HeaderAction>,
}
