use crate::message::{Message, SensitiveInput};
use crate::platform::tray::{TrayAction, TrayService};
use crate::state::{CredentialState, UiError, UsageState, WindowState};
use crate::{subscription, view, window as app_window};
use iced::{window, Element, Subscription, Task, Theme};
use volc_core::{
    ArkClient, CredentialStore, Credentials, KeyringCredentialStore, UsageReport, VolcError,
};

/// The single state container shared by every application window.
pub struct App {
    client: ArkClient,
    credential_store: KeyringCredentialStore,
    windows: WindowState,
    credentials: CredentialState,
    usage: UsageState,
    tray: Option<TrayService>,
    tray_error: Option<String>,
}

impl App {
    /// Initializes platform services, checks credentials, and opens the main window.
    pub fn boot() -> (Self, Task<Message>) {
        let (tray, tray_error) = match TrayService::new() {
            Ok(tray) => (Some(tray), None),
            Err(error) => {
                tracing::error!(%error, "tray initialization failed; close behavior is Exit");
                (None, Some(error))
            }
        };
        let (id, open) = app_window::main_window::open();
        let mut windows = WindowState::default();
        windows.set_main(id);
        let credentials = CredentialState {
            checking: true,
            ..CredentialState::default()
        };

        (
            Self {
                client: ArkClient::default(),
                credential_store: KeyringCredentialStore,
                windows,
                credentials,
                usage: UsageState::default(),
                tray,
                tray_error,
            },
            Task::batch([
                open.map(Message::MainWindowOpened),
                Task::perform(check_credentials(), Message::CredentialsChecked),
            ]),
        )
    }

    /// Applies one message and returns any asynchronous runtime work.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainWindowOpened(id) => {
                self.windows.set_main(id);
                Task::none()
            }
            Message::CloseRequested(id) if self.windows.main() == Some(id) => self.hide_main(),
            Message::CloseRequested(_) => Task::none(),
            Message::PollTray => self
                .tray
                .as_ref()
                .and_then(TrayService::try_recv)
                .map_or_else(Task::none, |action| self.update(Message::Tray(action))),
            Message::Tray(TrayAction::ShowMain) => self.show_main(),
            Message::Tray(TrayAction::HideMain) | Message::HideMain => self.hide_main(),
            Message::CredentialsChecked(result) => {
                self.credentials.checking = false;
                match result {
                    Ok(configured) => {
                        self.credentials.configured = configured;
                        self.credentials.error = None;
                        if configured {
                            return self.refresh();
                        }
                    }
                    Err(error) => self.credentials.error = Some(error),
                }
                Task::none()
            }
            Message::AccessKeyChanged(SensitiveInput(value)) => {
                self.credentials.access_key = value;
                Task::none()
            }
            Message::SecretKeyChanged(SensitiveInput(value)) => {
                self.credentials.secret_key = value;
                Task::none()
            }
            Message::SaveCredentials if !self.credentials.mutating => {
                self.credentials.mutating = true;
                self.credentials.error = None;
                let access_key = self.credentials.access_key.clone();
                let secret_key = self.credentials.secret_key.clone();
                Task::perform(
                    save_credentials(access_key, secret_key),
                    Message::CredentialsSaved,
                )
            }
            Message::SaveCredentials => Task::none(),
            Message::CredentialsSaved(result) => {
                self.credentials.mutating = false;
                match result {
                    Ok(()) => {
                        self.credentials.configured = true;
                        self.credentials.access_key.clear();
                        self.credentials.secret_key.clear();
                        self.credentials.error = None;
                        self.refresh()
                    }
                    Err(error) => {
                        self.credentials.error = Some(error);
                        Task::none()
                    }
                }
            }
            Message::ClearCredentials if !self.credentials.mutating => {
                self.credentials.mutating = true;
                self.credentials.error = None;
                Task::perform(clear_credentials(), Message::CredentialsCleared)
            }
            Message::ClearCredentials => Task::none(),
            Message::CredentialsCleared(result) => {
                self.credentials.mutating = false;
                match result {
                    Ok(()) => {
                        self.credentials.configured = false;
                        self.credentials.access_key.clear();
                        self.credentials.secret_key.clear();
                        self.credentials.error = None;
                        self.usage = UsageState::default();
                    }
                    Err(error) => self.credentials.error = Some(error),
                }
                Task::none()
            }
            Message::Refresh => self.refresh(),
            Message::UsageLoaded(result) => {
                self.usage.loading = false;
                match result {
                    Ok(report) => {
                        self.usage.report = Some(report);
                        self.usage.error = None;
                    }
                    Err(error) => self.usage.error = Some(error),
                }
                Task::none()
            }
            Message::LoadRaw if !self.usage.raw_loading && self.credentials.configured => {
                self.usage.raw_loading = true;
                self.usage.error = None;
                Task::perform(
                    fetch_raw(self.client.clone(), self.credential_store),
                    Message::RawLoaded,
                )
            }
            Message::LoadRaw => Task::none(),
            Message::RawLoaded(result) => {
                self.usage.raw_loading = false;
                match result {
                    Ok(raw) => {
                        self.usage.raw = Some(raw);
                        self.usage.error = None;
                    }
                    Err(error) => self.usage.error = Some(error),
                }
                Task::none()
            }
            Message::Tray(TrayAction::Quit) | Message::Exit => iced::exit(),
        }
    }

    /// Renders the requested window from shared state.
    pub fn view(&self, _id: window::Id) -> Element<'_, Message> {
        view::main(self)
    }

    /// Returns the title for a native window.
    pub fn title(&self, _id: window::Id) -> String {
        "VOLC Status".to_owned()
    }

    /// Returns passive runtime subscriptions.
    pub fn subscription(&self) -> Subscription<Message> {
        subscription::subscription(self)
    }

    /// Selects the built-in dark theme.
    pub fn theme(&self, _id: window::Id) -> Option<Theme> {
        Some(Theme::Dark)
    }

    /// Reports whether the native tray is usable.
    pub fn tray_available(&self) -> bool {
        self.tray.is_some()
    }

    /// Returns a bounded, non-sensitive tray initialization error.
    pub fn tray_error(&self) -> Option<&str> {
        self.tray_error.as_deref()
    }

    /// Returns credential UI state.
    pub fn credentials(&self) -> &CredentialState {
        &self.credentials
    }

    /// Returns the one shared usage state.
    pub fn usage(&self) -> &UsageState {
        &self.usage
    }

    fn refresh(&mut self) -> Task<Message> {
        if self.usage.loading || !self.credentials.configured {
            return Task::none();
        }
        self.usage.loading = true;
        self.usage.error = None;
        Task::perform(
            fetch_usage(self.client.clone(), self.credential_store),
            Message::UsageLoaded,
        )
    }

    fn show_main(&mut self) -> Task<Message> {
        if let Some(id) = self.windows.main() {
            window::gain_focus(id)
        } else {
            let (id, open) = app_window::main_window::open();
            self.windows.set_main(id);
            open.map(Message::MainWindowOpened)
        }
    }

    fn hide_main(&mut self) -> Task<Message> {
        let Some(id) = self.windows.take_main() else {
            return Task::none();
        };
        if self.tray.is_some() {
            window::close(id)
        } else {
            iced::exit()
        }
    }
}

async fn check_credentials() -> Result<bool, UiError> {
    match KeyringCredentialStore.load() {
        Ok(_) => Ok(true),
        Err(VolcError::CredentialsMissing) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

async fn save_credentials(access_key: String, secret_key: String) -> Result<(), UiError> {
    let credentials = Credentials::new(access_key, secret_key).map_err(UiError::from)?;
    KeyringCredentialStore
        .save(&credentials)
        .map_err(UiError::from)
}

async fn clear_credentials() -> Result<(), UiError> {
    KeyringCredentialStore.clear().map_err(UiError::from)
}

async fn fetch_usage(
    client: ArkClient,
    store: KeyringCredentialStore,
) -> Result<UsageReport, UiError> {
    let credentials = store.load().map_err(UiError::from)?;
    client
        .fetch_usage(&credentials)
        .await
        .map_err(UiError::from)
}

async fn fetch_raw(client: ArkClient, store: KeyringCredentialStore) -> Result<String, UiError> {
    let credentials = store.load().map_err(UiError::from)?;
    client
        .fetch_usage_raw(&credentials)
        .await
        .map_err(UiError::from)
}
