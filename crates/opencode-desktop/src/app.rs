use crate::config::{AppConfig, CloseBehavior, ConfigStore, FloatMode, FloatPosition};
use crate::message::{HeaderAction, Message, SensitiveInput, ThresholdField};
use crate::opencode_login;
use crate::platform::notification;
use crate::platform::tray::{TrayAction, TrayService};
use crate::state::{
    CredentialState, FloatState, MonitorState, SettingsState, UiError, UiState, UpdateProgress,
    UpdateState, UpdateStatus, UsageState, WindowState,
};
use crate::update::{check_for_update, download_task, install_update, open_url};
use crate::{subscription, theme, view, window as app_window};
use iced::keyboard::{key::Named, Key};
use iced::{clipboard, keyboard, window, Element, Subscription, Task, Theme};
use opencode_core::{
    evaluate_alerts, OpenCodeAuthStore, OpenCodeError, QuotaService, Thresholds, UsageReport,
};

/// The single state container shared by every application window.
pub struct App {
    service: QuotaService,
    windows: WindowState,
    credentials: CredentialState,
    usage: UsageState,
    settings: SettingsState,
    monitor: MonitorState,
    floating: FloatState,
    updater: UpdateState,
    ui: UiState,
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
                service: QuotaService::default(),
                windows,
                credentials,
                usage: UsageState::new(),
                settings: SettingsState::default(),
                monitor: MonitorState::default(),
                floating: FloatState::default(),
                updater: UpdateState::default(),
                ui: UiState::default(),
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
                #[cfg(target_os = "windows")]
                {
                    Task::batch([
                        app_window::float_window::round_corners(id).discard(),
                        app_window::float_window::restore_position(id, self.config.float_position)
                            .map(move |geometry| Message::FloatWindowGeometry(id, geometry)),
                    ])
                }
                #[cfg(not(target_os = "windows"))]
                {
                    window::monitor_size(id).map(move |size| Message::FloatMonitorSize(id, size))
                }
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
            // The window region does not follow a resize, so it is rebuilt for
            // every new size: float-mode switches and DPI changes both land here.
            #[cfg(target_os = "windows")]
            Message::WindowEvent(id, window::Event::Resized(_))
                if self.windows.floating() == Some(id) =>
            {
                app_window::float_window::round_corners(id).discard()
            }
            #[cfg(target_os = "windows")]
            Message::WindowEvent(id, window::Event::Moved(_))
                if self.windows.floating() == Some(id) =>
            {
                app_window::float_window::geometry(id)
                    .map(move |geometry| Message::FloatWindowGeometry(id, geometry))
            }
            #[cfg(not(target_os = "windows"))]
            Message::WindowEvent(id, window::Event::Moved(point))
                if self.windows.floating() == Some(id) =>
            {
                self.config.float_position = Some(FloatPosition {
                    x: point.x.round() as i32,
                    y: point.y.round() as i32,
                });
                self.floating.position_dirty = true;
                let top_docked =
                    app_window::float_window::is_top_docked(self.floating.top_docked, point.y, 0.0);
                if top_docked == self.floating.top_docked {
                    Task::none()
                } else {
                    self.floating.top_docked = top_docked;
                    let mode = self.float_mode();
                    let snap = top_docked.then(|| {
                        let snapped = iced::Point::new(point.x, 0.0);
                        self.config.float_position = Some(FloatPosition {
                            x: snapped.x.round() as i32,
                            y: snapped.y.round() as i32,
                        });
                        window::move_to(id, snapped)
                    });
                    Task::batch(
                        [Some(window::resize(id, mode.size())), snap]
                            .into_iter()
                            .flatten(),
                    )
                }
            }
            Message::WindowEvent(_, _) => Task::none(),
            Message::HeaderPressed(action) => {
                self.ui.header_focus = Some(action);
                self.update(action.message())
            }
            Message::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Tab),
                modifiers,
                ..
            }) if self.dashboard_open() => {
                self.ui.header_focus =
                    Some(next_header_focus(self.ui.header_focus, modifiers.shift()));
                Task::none()
            }
            Message::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Enter),
                ..
            }) if self.dashboard_open() && self.ui.header_focus.is_some() => self.update(
                Message::HeaderPressed(self.ui.header_focus.expect("focus checked above")),
            ),
            Message::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Escape),
                ..
            }) => {
                self.ui.debug_open = false;
                self.settings.open = false;
                Task::none()
            }
            Message::Keyboard(_) => Task::none(),
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
            #[cfg(target_os = "windows")]
            Message::FloatWindowGeometry(id, Some(geometry))
                if self.windows.floating() == Some(id) =>
            {
                self.config.float_position = Some(FloatPosition {
                    x: geometry.position.x.round() as i32,
                    y: geometry.position.y.round() as i32,
                });
                self.floating.position_dirty = true;
                let top_docked = app_window::float_window::is_top_docked_at_scale(
                    self.floating.top_docked,
                    geometry.position.y,
                    geometry.monitor_top,
                    geometry.scale_factor,
                );
                if top_docked == self.floating.top_docked {
                    Task::none()
                } else {
                    self.floating.top_docked = top_docked;
                    let mut tasks = vec![window::resize(id, self.float_mode().size())];
                    if top_docked {
                        tasks.push(app_window::float_window::snap_to_monitor_top(id).discard());
                    }
                    Task::batch(tasks)
                }
            }
            #[cfg(target_os = "windows")]
            Message::FloatWindowGeometry(_, _) => Task::none(),
            #[cfg(not(target_os = "windows"))]
            Message::FloatMonitorSize(id, Some(monitor)) if self.windows.floating() == Some(id) => {
                let Some(position) = self.config.float_position else {
                    return Task::none();
                };
                let point = app_window::float_window::clamp_position(
                    iced::Point::new(position.x as f32, position.y as f32),
                    monitor,
                    self.float_mode().size(),
                );
                self.config.float_position = Some(FloatPosition {
                    x: point.x.round() as i32,
                    y: point.y.round() as i32,
                });
                Task::batch([window::move_to(id, point), self.persist_config_silently()])
            }
            #[cfg(not(target_os = "windows"))]
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
                        let refresh_task = if self.configured() && self.config.monitor_enabled {
                            self.refresh()
                        } else {
                            Task::none()
                        };
                        let update_task = if self.config.update_checks_enabled {
                            self.start_update_check()
                        } else {
                            Task::none()
                        };
                        return Task::batch([float_task, refresh_task, update_task]);
                    }
                    Err(error) => self.settings.error = Some(error),
                }
                Task::none()
            }
            Message::OpenSettings => {
                self.ui.debug_open = false;
                self.settings.open = true;
                self.settings.error = None;
                self.settings.notice = None;
                Task::none()
            }
            Message::CloseSettings => {
                self.settings.open = false;
                Task::none()
            }
            Message::CloseOverlay => {
                self.ui.debug_open = false;
                Task::none()
            }
            Message::CopyRaw => {
                let Some(raw) = self.usage.raw.clone() else {
                    return Task::none();
                };
                self.ui.toast = Some("原始 JSON 已复制到剪贴板。".to_owned());
                clipboard::write(raw)
            }
            Message::DismissToast => {
                self.ui.toast = None;
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
                    Ok(available) => {
                        self.credentials.opencode = available;
                        self.credentials.error = None;
                        if self.configured() {
                            return self.refresh();
                        }
                    }
                    Err(error) => self.credentials.error = Some(error),
                }
                Task::none()
            }
            Message::OpenCodeWorkspaceChanged(value) => {
                self.credentials.opencode_workspace = value;
                Task::none()
            }
            Message::OpenCodeCookieChanged(SensitiveInput(value)) => {
                self.credentials.opencode_cookie = value;
                Task::none()
            }
            Message::StartOpenCodeLogin => {
                self.credentials.error = None;
                match opencode_login::open_login_page() {
                    Ok(()) => {
                        self.credentials.login_notice = Some(
                            "已在浏览器中打开 opencode.ai 登录页。请完成 GitHub / Google 登录,打开你的工作区,将地址栏的 Workspace ID 填入上方,并在浏览器开发者工具中复制 auth Cookie 填入下方,再点击保存。"
                                .to_owned(),
                        );
                    }
                    Err(error) => {
                        tracing::warn!(%error, "failed to open OpenCode login page");
                        self.credentials.error = Some(UiError {
                            user: "无法打开浏览器,请手动访问 opencode.ai/auth 完成登录。"
                                .to_owned(),
                            detail: format!("failed to open browser: {error}"),
                        });
                    }
                }
                Task::none()
            }
            Message::SaveOpenCodeCredentials if !self.credentials.mutating => {
                let workspace = self.credentials.opencode_workspace.trim().to_owned();
                if workspace.is_empty() {
                    self.credentials.error = Some(UiError {
                        user: "Workspace ID 不能为空。".to_owned(),
                        detail: "empty OpenCode workspace id".to_owned(),
                    });
                    return Task::none();
                }
                self.credentials.mutating = true;
                self.credentials.error = None;
                let cookie = self.credentials.opencode_cookie.clone();
                Task::perform(
                    save_opencode_credentials(cookie),
                    Message::OpenCodeCredentialsSaved,
                )
            }
            Message::SaveOpenCodeCredentials => Task::none(),
            Message::OpenCodeCredentialsSaved(result) => {
                self.credentials.mutating = false;
                match result {
                    Ok(()) => {
                        self.credentials.opencode = true;
                        let workspace = self.credentials.opencode_workspace.trim().to_owned();
                        // Keep the non-sensitive workspace visible; clear the
                        // cookie so the saved secret is never shown back.
                        self.credentials.opencode_workspace = workspace.clone();
                        self.credentials.opencode_cookie.clear();
                        self.credentials.error = None;
                        self.config.opencode_workspace_id = Some(workspace);
                        Task::batch([self.persist_config_silently(), self.refresh()])
                    }
                    Err(error) => {
                        self.credentials.error = Some(error);
                        Task::none()
                    }
                }
            }
            Message::ClearCredentials if !self.credentials.mutating => {
                self.ui.confirm_clear_credentials = true;
                Task::none()
            }
            Message::ClearCredentials => Task::none(),
            Message::ConfirmClearCredentials if !self.credentials.mutating => {
                self.credentials.mutating = true;
                self.credentials.error = None;
                Task::perform(clear_opencode_cookie(), Message::OpenCodeCredentialsCleared)
            }
            Message::ConfirmClearCredentials => Task::none(),
            Message::CancelClearCredentials => {
                self.ui.confirm_clear_credentials = false;
                Task::none()
            }
            Message::OpenCodeCredentialsCleared(result) => {
                self.credentials.mutating = false;
                self.ui.confirm_clear_credentials = false;
                match result {
                    Ok(()) => {
                        self.credentials.opencode = false;
                        self.credentials.opencode_workspace.clear();
                        self.credentials.opencode_cookie.clear();
                        self.credentials.error = None;
                        self.config.opencode_workspace_id = None;
                        self.config.monitor_enabled = false;
                        self.usage = UsageState::new();
                        self.monitor = MonitorState::default();
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
            Message::LoadRaw if !self.usage.raw_loading && self.configured() => {
                self.settings.open = false;
                self.ui.debug_open = true;
                self.usage.raw_loading = true;
                self.usage.error = None;
                let workspace = self
                    .config
                    .opencode_workspace_id
                    .clone()
                    .unwrap_or_default();
                let service = self.service.clone();
                Task::perform(fetch_opencode_raw(service, workspace), Message::RawLoaded)
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
            Message::CheckForUpdate if self.updater.status.busy() => Task::none(),
            Message::CheckForUpdate => self.start_update_check(),
            Message::UpdateCheckTick => {
                if self.config.update_checks_enabled && !self.updater.status.busy() {
                    self.start_update_check()
                } else {
                    Task::none()
                }
            }
            Message::UpdateChecked(result) => {
                let start = self.updater.apply_check_result(
                    chrono::Utc::now().timestamp_millis(),
                    result,
                    self.config.auto_download_updates,
                );
                if start {
                    return self.start_download();
                }
                Task::none()
            }
            Message::UpdateDownloadProgress { downloaded, total } => {
                self.updater.progress = Some(UpdateProgress { downloaded, total });
                Task::none()
            }
            Message::DownloadUpdate if !self.updater.status.busy() => self.start_download(),
            Message::DownloadUpdate => Task::none(),
            Message::UpdateDownloaded(result) => {
                self.updater.apply_download_result(result);
                Task::none()
            }
            Message::InstallUpdate => {
                let Some(package) = self.updater.downloaded.clone() else {
                    return Task::none();
                };
                self.updater.status = UpdateStatus::Installing;
                self.updater.error = None;
                Task::perform(
                    async move { install_update(&package) },
                    Message::UpdateInstallStarted,
                )
            }
            Message::UpdateInstallStarted(result) => match result {
                Ok(true) => iced::exit(),
                Ok(false) => {
                    self.updater.status = UpdateStatus::UpToDate;
                    self.updater.available = None;
                    self.updater.downloaded = None;
                    self.settings.notice = Some("更新包已打开，请按提示完成安装。".to_owned());
                    Task::none()
                }
                Err(error) => {
                    self.updater.error = Some(error);
                    self.updater.status = UpdateStatus::Error;
                    Task::none()
                }
            },
            Message::DismissUpdate => {
                self.updater.banner_dismissed = true;
                Task::none()
            }
            Message::OpenReleaseNotes => {
                if let Some(info) = &self.updater.available {
                    if let Err(error) = open_url(&info.release_notes_url) {
                        self.updater.error = Some(UiError {
                            user: "无法打开更新说明。".to_owned(),
                            detail: error,
                        });
                    }
                }
                Task::none()
            }
            Message::UpdateChecksEnabledChanged(enabled) => {
                self.config.update_checks_enabled = enabled;
                let check = if enabled && !self.updater.status.busy() {
                    self.start_update_check()
                } else {
                    Task::none()
                };
                Task::batch([self.persist_config_silently(), check])
            }
            Message::AutoDownloadUpdatesChanged(enabled) => {
                self.config.auto_download_updates = enabled;
                self.persist_config_silently()
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
            "OpenCode Quota Checker · 悬浮窗".to_owned()
        } else {
            "OpenCode Quota Checker".to_owned()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        subscription::subscription(self)
    }

    pub fn theme(&self, id: window::Id) -> Option<Theme> {
        if self.windows.floating() == Some(id) {
            Some(theme::floating())
        } else {
            Some(theme::application())
        }
    }

    pub fn window_style(&self, theme: &Theme) -> iced::theme::Style {
        theme::window_style(theme)
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

    pub fn float_mode(&self) -> FloatMode {
        if self.floating.top_docked {
            FloatMode::Docked
        } else {
            self.config.float_mode
        }
    }

    pub fn config_loaded(&self) -> bool {
        self.config_loaded
    }

    pub fn ui(&self) -> &UiState {
        &self.ui
    }

    pub fn updater(&self) -> &UpdateState {
        &self.updater
    }

    pub fn update_checks_enabled(&self) -> bool {
        self.config.update_checks_enabled
    }

    pub fn has_report(&self) -> bool {
        self.usage.report.is_some()
    }

    pub fn monitor_interval(&self) -> Option<u64> {
        (self.config_loaded && self.configured() && self.config.monitor_enabled)
            .then_some(self.config.monitor_interval_secs)
    }

    pub fn float_position_dirty(&self) -> bool {
        self.floating.position_dirty
    }

    pub fn confirm_clear_credentials(&self) -> bool {
        self.ui.confirm_clear_credentials
    }

    pub fn toast_visible(&self) -> bool {
        self.ui.toast.is_some()
    }

    fn dashboard_open(&self) -> bool {
        !self.ui.debug_open
            && !self.settings.open
            && !self.credentials.checking
            && self.config_loaded
            && self.configured()
    }

    /// Reports whether the OpenCode workspace and auth cookie are configured.
    pub fn configured(&self) -> bool {
        self.credentials.opencode
            && self
                .config
                .opencode_workspace_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    fn apply_config(&mut self, config: AppConfig) {
        let mut config = config;
        if config.float_mode == FloatMode::Docked {
            config.float_mode = FloatMode::Compact;
        }
        self.settings.interval = config.monitor_interval_secs.to_string();
        self.settings.five_hour = config.thresholds.five_hour.to_string();
        self.settings.weekly = config.thresholds.weekly.to_string();
        self.settings.monthly = config.thresholds.monthly.to_string();
        self.credentials.opencode_workspace =
            config.opencode_workspace_id.clone().unwrap_or_default();
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
        if self.usage.loading || !self.configured() {
            return Task::none();
        }
        self.usage.loading = true;
        self.usage.error = None;
        let workspace = self
            .config
            .opencode_workspace_id
            .clone()
            .unwrap_or_default();
        let service = self.service.clone();
        Task::perform(
            fetch_opencode_usage(service, workspace),
            Message::UsageLoaded,
        )
    }

    fn start_update_check(&mut self) -> Task<Message> {
        self.updater.status = UpdateStatus::Checking;
        self.updater.error = None;
        Task::perform(check_for_update(), Message::UpdateChecked)
    }

    fn start_download(&mut self) -> Task<Message> {
        let Some(info) = self.updater.available.clone() else {
            return Task::none();
        };
        self.updater.begin_download();
        download_task(info)
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
        self.floating.top_docked = self.config.float_position.is_some_and(|position| {
            app_window::float_window::is_top_docked(false, position.y as f32, 0.0)
        });
        let (id, open) =
            app_window::float_window::open(self.float_mode(), self.config.float_position);
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
        self.floating.top_docked = mode == FloatMode::Docked;
        if mode != FloatMode::Docked {
            self.config.float_mode = mode;
        }
        let effective_mode = self.float_mode();
        let resize = self
            .windows
            .floating()
            .map_or_else(Task::none, |id| window::resize(id, effective_mode.size()));
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

fn next_header_focus(current: Option<HeaderAction>, reverse: bool) -> HeaderAction {
    let actions = HeaderAction::ALL.to_vec();
    let current_index =
        current.and_then(|action| actions.iter().position(|candidate| *candidate == action));
    let next_index = match (current_index, reverse) {
        (Some(0), true) | (None, true) => actions.len() - 1,
        (Some(index), true) => index - 1,
        (Some(index), false) => (index + 1) % actions.len(),
        (None, false) => 0,
    };
    actions[next_index]
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
        .and_then(|store| store.load_or_default())
        .map_err(UiError::from)
}

async fn save_config(config: AppConfig) -> Result<AppConfig, UiError> {
    ConfigStore::discover()
        .and_then(|store| store.save(&config))
        .map_err(UiError::from)?;
    Ok(config)
}

async fn check_credentials() -> Result<bool, UiError> {
    match OpenCodeAuthStore.load() {
        Ok(_) => Ok(true),
        Err(OpenCodeError::CredentialsMissing) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

async fn save_opencode_credentials(cookie: String) -> Result<(), UiError> {
    OpenCodeAuthStore.save(&cookie).map_err(UiError::from)
}

async fn clear_opencode_cookie() -> Result<(), UiError> {
    OpenCodeAuthStore.clear().map_err(UiError::from)
}

async fn fetch_opencode_usage(
    service: QuotaService,
    workspace_id: String,
) -> Result<UsageReport, UiError> {
    let cookie = OpenCodeAuthStore.load().map_err(UiError::from)?;
    service
        .fetch_quota(&workspace_id, &cookie)
        .await
        .map_err(UiError::from)
}

async fn fetch_opencode_raw(
    service: QuotaService,
    workspace_id: String,
) -> Result<String, UiError> {
    let cookie = OpenCodeAuthStore.load().map_err(UiError::from)?;
    service
        .fetch_raw_dashboard(&workspace_id, &cookie)
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
