use crate::config::{AppConfig, CloseBehavior, ConfigStore, FloatMode, FloatPosition};
use crate::message::{Message, SensitiveInput, ThresholdField};
use crate::platform::notification;
use crate::platform::tray::{TrayAction, TrayService};
use crate::state::{
    CredentialState, FloatState, MonitorState, SettingsState, UiError, UsageState, WindowState,
};
use crate::{subscription, view, window as app_window};
use iced::{window, Element, Subscription, Task, Theme};
use volc_core::{
    evaluate_alerts, ArkClient, CredentialStore, Credentials, KeyringCredentialStore, Thresholds,
    UsageReport, VolcError,
};

/// The single state container shared by every application window.
pub struct App {
    client: ArkClient,
    credential_store: KeyringCredentialStore,
    windows: WindowState,
    credentials: CredentialState,
    usage: UsageState,
    settings: SettingsState,
    monitor: MonitorState,
    floating: FloatState,
    config: AppConfig,
    config_loaded: bool,
    tray: Option<TrayService>,
    tray_error: Option<String>,
}

impl App {
    /// Initializes platform services, loads config, checks credentials, and opens the main window.
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
                usage: UsageState::new(),
                settings: SettingsState::default(),
                monitor: MonitorState::default(),
                floating: FloatState::default(),
                config: AppConfig::default(),
                config_loaded: false,
                tray,
                tray_error,
            },
            Task::batch([
                open.map(Message::MainWindowOpened),
                Task::perform(check_credentials(), Message::CredentialsChecked),
                Task::perform(load_config(), Message::ConfigLoaded),
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
            Message::FloatWindowOpened(id) => {
                self.windows.set_floating(id);
                window::monitor_size(id).map(move |size| Message::FloatMonitorSize(id, size))
            }
            Message::CloseRequested(id) if self.windows.main() == Some(id) => {
                if self.tray.is_none() || self.config.close_behavior == CloseBehavior::Exit {
                    iced::exit()
                } else {
                    self.hide_main()
                }
            }
            Message::CloseRequested(id) if self.windows.floating() == Some(id) => {
                self.close_float()
            }
            Message::CloseRequested(_) => Task::none(),
            Message::WindowEvent(id, window::Event::Moved(point))
                if self.windows.floating() == Some(id) =>
            {
                self.config.float_position = Some(FloatPosition {
                    x: point.x.round() as i32,
                    y: point.y.round() as i32,
                });
                self.floating.position_dirty = true;
                Task::none()
            }
            Message::WindowEvent(_, _) => Task::none(),
            Message::PollTray => self
                .tray
                .as_ref()
                .and_then(TrayService::try_recv)
                .map_or_else(Task::none, |action| self.update(Message::Tray(action))),
            Message::Tray(TrayAction::ShowMain) => self.show_main(),
            Message::Tray(TrayAction::HideMain) | Message::HideMain => self.hide_main(),
            Message::Tray(TrayAction::ToggleFloat) | Message::ToggleFloat => {
                if self.windows.floating().is_some() {
                    self.close_float()
                } else {
                    self.open_float()
                }
            }
            Message::CloseFloat => self.close_float(),
            Message::FloatModeChanged(mode) => self.change_float_mode(mode),
            Message::DragFloat => self
                .windows
                .floating()
                .map_or_else(Task::none, window::drag),
            Message::FloatMonitorSize(id, Some(monitor)) if self.windows.floating() == Some(id) => {
                let Some(position) = self.config.float_position else {
                    return Task::none();
                };
                let point = app_window::float_window::clamp_position(
                    iced::Point::new(position.x as f32, position.y as f32),
                    monitor,
                    self.config.float_mode.size(),
                );
                self.config.float_position = Some(FloatPosition {
                    x: point.x.round() as i32,
                    y: point.y.round() as i32,
                });
                Task::batch([window::move_to(id, point), self.persist_config_silently()])
            }
            Message::FloatMonitorSize(_, _) => Task::none(),
            Message::PersistFloatPosition if self.floating.position_dirty => {
                self.floating.position_dirty = false;
                self.persist_config_silently()
            }
            Message::PersistFloatPosition => Task::none(),
            Message::ConfigPersisted(result) => {
                if let Ok(config) = result {
                    self.config = config;
                }
                Task::none()
            }
            Message::ConfigLoaded(result) => {
                self.config_loaded = true;
                match result {
                    Ok(config) => {
                        self.apply_config(config);
                        let float_task = if self.config.float_open {
                            self.open_float()
                        } else {
                            Task::none()
                        };
                        let refresh_task =
                            if self.credentials.configured && self.config.monitor_enabled {
                                self.refresh()
                            } else {
                                Task::none()
                            };
                        return Task::batch([float_task, refresh_task]);
                    }
                    Err(error) => self.settings.error = Some(error),
                }
                Task::none()
            }
            Message::OpenSettings => {
                self.settings.open = true;
                self.settings.error = None;
                self.settings.notice = None;
                Task::none()
            }
            Message::CloseSettings => {
                self.settings.open = false;
                Task::none()
            }
            Message::IntervalChanged(value) => {
                self.settings.interval = value;
                Task::none()
            }
            Message::ThresholdChanged(field, value) => {
                match field {
                    ThresholdField::FiveHour => self.settings.five_hour = value,
                    ThresholdField::Weekly => self.settings.weekly = value,
                    ThresholdField::Monthly => self.settings.monthly = value,
                }
                Task::none()
            }
            Message::StartMonitor if !self.settings.saving => {
                let config = match self.config_from_settings(true) {
                    Ok(config) => config,
                    Err(error) => {
                        self.settings.error = Some(error);
                        return Task::none();
                    }
                };
                self.settings.saving = true;
                self.settings.error = None;
                Task::perform(save_config(config), Message::ConfigSaved)
            }
            Message::StopMonitor if !self.settings.saving => {
                let mut config = self.config.clone();
                config.monitor_enabled = false;
                self.settings.saving = true;
                self.settings.error = None;
                Task::perform(save_config(config), Message::ConfigSaved)
            }
            Message::StartMonitor | Message::StopMonitor => Task::none(),
            Message::ConfigSaved(result) => {
                self.settings.saving = false;
                match result {
                    Ok(config) => {
                        let enabled = config.monitor_enabled;
                        self.apply_config(config);
                        self.settings.notice = Some(if enabled {
                            "监控配置已保存并启动。".to_owned()
                        } else {
                            "监控已停止。".to_owned()
                        });
                        if enabled {
                            return self.refresh();
                        }
                    }
                    Err(error) => self.settings.error = Some(error),
                }
                Task::none()
            }
            Message::MonitorTick => self.refresh(),
            Message::NotificationsDelivered(result) => {
                self.monitor.notification_error = result.err();
                Task::none()
            }
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
                        self.usage = UsageState::new();
                        self.monitor = MonitorState::default();
                        self.config.monitor_enabled = false;
                        return Task::perform(
                            save_config(self.config.clone()),
                            Message::ConfigSaved,
                        );
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
                        let notification_task = self.alert_task(&report);
                        self.usage.report = Some(report);
                        self.usage.error = None;
                        return notification_task;
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
            Message::Tick(now_ms) => {
                self.usage.now_ms = now_ms;
                Task::none()
            }
            Message::Tray(TrayAction::Quit) | Message::Exit => iced::exit(),
        }
    }

    pub fn view(&self, id: window::Id) -> Element<'_, Message> {
        if self.windows.floating() == Some(id) {
            view::floating(self)
        } else {
            view::main(self)
        }
    }

    pub fn title(&self, id: window::Id) -> String {
        if self.windows.floating() == Some(id) {
            "VOLC Status · 悬浮窗".to_owned()
        } else {
            "VOLC Status".to_owned()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        subscription::subscription(self)
    }

    pub fn theme(&self, _id: window::Id) -> Option<Theme> {
        Some(Theme::Dark)
    }

    pub fn tray_available(&self) -> bool {
        self.tray.is_some()
    }

    pub fn tray_error(&self) -> Option<&str> {
        self.tray_error.as_deref()
    }

    pub fn credentials(&self) -> &CredentialState {
        &self.credentials
    }

    pub fn usage(&self) -> &UsageState {
        &self.usage
    }

    pub fn settings(&self) -> &SettingsState {
        &self.settings
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn config_loaded(&self) -> bool {
        self.config_loaded
    }

    pub fn has_report(&self) -> bool {
        self.usage.report.is_some()
    }

    pub fn monitor_interval(&self) -> Option<u64> {
        (self.config_loaded && self.credentials.configured && self.config.monitor_enabled)
            .then_some(self.config.monitor_interval_secs)
    }

    pub fn float_position_dirty(&self) -> bool {
        self.floating.position_dirty
    }

    fn apply_config(&mut self, config: AppConfig) {
        self.settings.interval = config.monitor_interval_secs.to_string();
        self.settings.five_hour = config.thresholds.five_hour.to_string();
        self.settings.weekly = config.thresholds.weekly.to_string();
        self.settings.monthly = config.thresholds.monthly.to_string();
        self.config = config;
    }

    fn config_from_settings(&self, enabled: bool) -> Result<AppConfig, UiError> {
        let interval = parse_u64(&self.settings.interval, "轮询间隔")?;
        let thresholds = Thresholds {
            five_hour: parse_f64(&self.settings.five_hour, "5 小时阈值")?,
            weekly: parse_f64(&self.settings.weekly, "近一周阈值")?,
            monthly: parse_f64(&self.settings.monthly, "近一月阈值")?,
        };
        let config = AppConfig {
            monitor_enabled: enabled,
            monitor_interval_secs: interval,
            thresholds,
            ..self.config.clone()
        };
        config.validate().map_err(UiError::from)
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

    fn alert_task(&mut self, report: &UsageReport) -> Task<Message> {
        if !self.config.monitor_enabled {
            return Task::none();
        }
        let evaluation =
            evaluate_alerts(report, &self.config.thresholds, &self.monitor.last_alerted);
        self.monitor.last_alerted = evaluation.next_alerted;
        if evaluation.decisions.is_empty() {
            Task::none()
        } else {
            Task::perform(
                async move { notification::deliver(evaluation.decisions) },
                Message::NotificationsDelivered,
            )
        }
    }

    fn open_float(&mut self) -> Task<Message> {
        if let Some(id) = self.windows.floating() {
            return window::gain_focus(id);
        }
        self.config.float_open = true;
        if let Some(tray) = &self.tray {
            tray.set_float_open(true);
        }
        let (id, open) =
            app_window::float_window::open(self.config.float_mode, self.config.float_position);
        self.windows.set_floating(id);
        Task::batch([
            open.map(Message::FloatWindowOpened),
            self.persist_config_silently(),
        ])
    }

    fn close_float(&mut self) -> Task<Message> {
        let Some(id) = self.windows.take_floating() else {
            return Task::none();
        };
        self.config.float_open = false;
        if let Some(tray) = &self.tray {
            tray.set_float_open(false);
        }
        Task::batch([window::close(id), self.persist_config_silently()])
    }

    fn change_float_mode(&mut self, mode: FloatMode) -> Task<Message> {
        self.config.float_mode = mode;
        let resize = self
            .windows
            .floating()
            .map_or_else(Task::none, |id| window::resize(id, mode.size()));
        Task::batch([resize, self.persist_config_silently()])
    }

    fn persist_config_silently(&self) -> Task<Message> {
        Task::perform(save_config(self.config.clone()), Message::ConfigPersisted)
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

fn parse_u64(value: &str, label: &str) -> Result<u64, UiError> {
    value.trim().parse::<u64>().map_err(|_| UiError {
        user: format!("{label}必须是整数。"),
        detail: format!("invalid integer for {label}"),
    })
}

fn parse_f64(value: &str, label: &str) -> Result<f64, UiError> {
    value.trim().parse::<f64>().map_err(|_| UiError {
        user: format!("{label}必须是数字。"),
        detail: format!("invalid number for {label}"),
    })
}

async fn load_config() -> Result<AppConfig, UiError> {
    ConfigStore::discover()
        .and_then(|store| store.load_or_migrate())
        .map_err(UiError::from)
}

async fn save_config(config: AppConfig) -> Result<AppConfig, UiError> {
    ConfigStore::discover()
        .and_then(|store| store.save(&config))
        .map_err(UiError::from)?;
    Ok(config)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_validation_accepts_boundaries() {
        assert_eq!(parse_u64("30", "interval").expect("valid integer"), 30);
        assert_eq!(parse_f64("100", "threshold").expect("valid number"), 100.0);
    }

    #[test]
    fn settings_validation_rejects_text() {
        assert!(parse_u64("later", "interval").is_err());
        assert!(parse_f64("high", "threshold").is_err());
    }
}
