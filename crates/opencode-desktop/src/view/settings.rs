use crate::config::AppConfig;
use crate::message::{Message, SensitiveInput, ThresholdField};
use crate::state::{CredentialState, SettingsState, UpdateState, UpdateStatus};
use crate::theme;
use crate::view::components::{icon_button, icons, settings, status_badge};
use iced::alignment::Horizontal;
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input, toggler, Column,
};
use iced::{Element, Fill, Length};

/// Settings page: manage the OpenCode connection and quota-monitoring rules.
///
/// The page is a centered, fixed-width column so it reads as a focused
/// settings dialog rather than a full-bleed form.
pub fn view<'a>(
    state: &'a SettingsState,
    config: &AppConfig,
    credentials: &'a CredentialState,
    updater: &'a UpdateState,
) -> Element<'a, Message> {
    let mut children: Vec<Element<'a, Message>> = vec![
        header(),
        account_card(credentials),
        monitor_card(state, config),
        update_card(config, updater),
        settings::danger_zone(!credentials.mutating),
    ];

    // Page-level feedback appears right under the header so it stays in view.
    if let Some(error) = &state.error {
        children.insert(
            1,
            settings::notice(settings::NoticeKind::Error, &error.user),
        );
    } else if let Some(notice) = &state.notice {
        children.insert(1, settings::notice(settings::NoticeKind::Success, notice));
    }

    scrollable(
        container(
            Column::with_children(children)
                .spacing(16)
                .width(Fill)
                .max_width(680.0),
        )
        .width(Fill)
        .padding(28)
        .align_x(Horizontal::Center),
    )
    .width(Fill)
    .height(Fill)
    .into()
}

/// Settings page header: title, subtitle and a ghost close button.
fn header() -> Element<'static, Message> {
    container(
        column![
            row![
                text("设置").size(24).color(theme::palette::TEXT_PRIMARY),
                row![].width(Fill),
                icon_button::view(icons::CLOSE, "关闭", Message::CloseSettings, false),
            ]
            .spacing(16)
            .align_y(iced::Alignment::Center),
            text("管理 OpenCode 连接和额度监控规则")
                .size(13)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(4),
    )
    .width(Fill)
    .padding(iced::Padding::default().bottom(8.0))
    .into()
}

/// OpenCode Go connection card: workspace ID + auth cookie + security hint.
fn account_card<'a>(credentials: &'a CredentialState) -> Element<'a, Message> {
    let workspace = text_input("ws_xxxxxxxxxxxxx", &credentials.opencode_workspace)
        .on_input(Message::OpenCodeWorkspaceChanged)
        .padding([10, 12])
        .style(theme::settings_input);
    let cookie = text_input("粘贴 auth Cookie", &credentials.opencode_cookie)
        .on_input(|value| Message::OpenCodeCookieChanged(SensitiveInput(value)))
        .secure(true)
        .padding([10, 12])
        .style(theme::settings_input);

    let security_hint = column![
        text("🔒 凭证安全保存在系统钥匙串中")
            .size(theme::typography::BODY)
            .color(theme::palette::TEXT_SECONDARY),
        text("仅在请求 OpenCode API 时使用。")
            .size(theme::typography::CAPTION)
            .color(theme::palette::TEXT_MUTED),
    ]
    .spacing(2);

    let can_save = !credentials.mutating
        && !credentials.opencode_workspace.trim().is_empty()
        && !credentials.opencode_cookie.trim().is_empty();
    let save = if can_save {
        button("保存连接")
            .on_press(Message::SaveOpenCodeCredentials)
            .style(button::primary)
            .padding([10, 20])
    } else {
        button(if credentials.mutating {
            "保存中…"
        } else {
            "保存连接"
        })
        .padding([10, 20])
    };

    let mut card_content = column![
        settings::card_header(icons::GLOBE, "OpenCode Go", "OpenCode 账户连接", None),
        settings::form_field("Workspace ID", workspace.into(), None),
        settings::form_field("Auth Cookie", cookie.into(), None),
        security_hint,
        container(save).width(Fill).align_x(Horizontal::Right),
    ]
    .spacing(14);

    if let Some(error) = &credentials.error {
        card_content =
            card_content.push(settings::notice(settings::NoticeKind::Error, &error.user));
    }

    settings::settings_card(card_content).into()
}

/// Monitor card: status badge, polling interval, thresholds and action button.
fn monitor_card<'a>(state: &'a SettingsState, config: &AppConfig) -> Element<'a, Message> {
    let badge = if config.monitor_enabled {
        status_badge::view("● 运行中", status_badge::Tone::Success)
    } else {
        status_badge::view("● 已停止", status_badge::Tone::Neutral)
    };

    let interval = settings::form_field(
        "检查间隔",
        row![
            text("每")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_SECONDARY),
            text_input("", &state.interval)
                .on_input(Message::IntervalChanged)
                .padding([10, 12])
                .style(theme::settings_input)
                .width(Length::Fixed(84.0)),
            text("秒")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_SECONDARY),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into(),
        Some("允许范围 30–3600 秒"),
    );

    let thresholds = column![
        text("通知阈值")
            .size(theme::typography::BODY)
            .color(theme::palette::TEXT_SECONDARY),
        row![
            settings::threshold_field("5 小时", &state.five_hour, |value| {
                Message::ThresholdChanged(ThresholdField::FiveHour, value)
            }),
            settings::threshold_field("近一周", &state.weekly, |value| {
                Message::ThresholdChanged(ThresholdField::Weekly, value)
            }),
            settings::threshold_field("近一月", &state.monthly, |value| {
                Message::ThresholdChanged(ThresholdField::Monthly, value)
            }),
        ]
        .spacing(12),
    ]
    .spacing(6);

    let action = if state.saving {
        button("保存中…").padding([10, 20])
    } else if config.monitor_enabled {
        button("停止监控")
            .on_press(Message::StopMonitor)
            .style(theme::secondary_button)
            .padding([10, 20])
    } else {
        button("保存并启动")
            .on_press(Message::StartMonitor)
            .style(button::primary)
            .padding([10, 20])
    };

    settings::settings_card(
        column![
            settings::card_header(
                icons::ACTIVITY,
                "额度监控",
                "配置自动检查频率和通知阈值",
                Some(badge),
            ),
            interval,
            thresholds,
            container(action).width(Fill).align_x(Horizontal::Right),
        ]
        .spacing(14),
    )
    .into()
}

/// Application update card: current version, check/download toggles, manual
/// check, and the per-status install actions.
fn update_card(config: &AppConfig, updater: &UpdateState) -> Element<'static, Message> {
    let status = match updater.status {
        UpdateStatus::Checking => status_badge::view("检查中…", status_badge::Tone::Primary),
        UpdateStatus::Downloading => status_badge::view("下载中…", status_badge::Tone::Primary),
        UpdateStatus::UpToDate => status_badge::view("已是最新", status_badge::Tone::Success),
        UpdateStatus::Error => status_badge::view("检查失败", status_badge::Tone::Danger),
        _ => status_badge::view("未检查", status_badge::Tone::Neutral),
    };

    let mut content = column![
        settings::card_header(
            icons::REFRESH,
            "应用更新",
            "从 GitHub Releases 自动发现并安装新版本",
            Some(status),
        ),
        settings::form_field(
            "当前版本",
            text(format!("v{}", env!("CARGO_PKG_VERSION")))
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_PRIMARY)
                .into(),
            None,
        ),
        toggle_row(
            "自动检查更新",
            config.update_checks_enabled,
            |enabled| { Message::UpdateChecksEnabledChanged(enabled) }
        ),
        toggle_row(
            "自动下载更新",
            config.auto_download_updates,
            |enabled| { Message::AutoDownloadUpdatesChanged(enabled) }
        ),
        settings::form_field(
            "上次检查",
            text(last_checked_text(updater.last_checked_at))
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_SECONDARY)
                .into(),
            None,
        ),
        row![
            row![].width(Fill),
            button("检查更新")
                .on_press(Message::CheckForUpdate)
                .style(theme::secondary_button)
                .padding([8, 16]),
        ]
        .align_y(iced::Alignment::Center),
    ]
    .spacing(14);

    if let Some(status_content) = update_status_content(updater) {
        content = content.push(status_content);
    }

    settings::settings_card(content).into()
}

/// Label on the left, toggle on the right, matching the settings aesthetic.
fn toggle_row<'a>(
    label: &'static str,
    value: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label)
            .size(theme::typography::BODY)
            .color(theme::palette::TEXT_SECONDARY),
        row![].width(Fill),
        toggler(value).on_toggle(on_toggle),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

/// Status-specific block rendered under the update card controls.
fn update_status_content(updater: &UpdateState) -> Option<Element<'static, Message>> {
    match updater.status {
        UpdateStatus::Checking => Some(
            text("正在检查更新…")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_SECONDARY)
                .into(),
        ),
        UpdateStatus::Available => {
            let tag = updater
                .available
                .as_ref()
                .map_or("新版本".to_owned(), |info| info.tag.clone());
            Some(
                row![
                    text(format!("新版本 {tag} 可用"))
                        .size(theme::typography::BODY)
                        .color(theme::palette::TEXT_PRIMARY),
                    row![].width(Fill),
                    button("查看更新说明")
                        .on_press(Message::OpenReleaseNotes)
                        .style(theme::soft_button)
                        .padding([8, 16]),
                    button("下载更新")
                        .on_press(Message::DownloadUpdate)
                        .style(button::primary)
                        .padding([8, 16]),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .into(),
            )
        }
        UpdateStatus::Downloading => {
            let tag = updater
                .available
                .as_ref()
                .map_or("更新".to_owned(), |info| info.tag.clone());
            let progress = updater
                .progress
                .and_then(|progress| {
                    progress
                        .total
                        .map(|total| progress.downloaded as f32 / total as f32)
                })
                .unwrap_or(0.0);
            Some(
                column![
                    text(format!("正在下载 {tag}…"))
                        .size(theme::typography::BODY)
                        .color(theme::palette::TEXT_SECONDARY),
                    progress_bar(0.0..=1.0, progress.clamp(0.0, 1.0))
                        .girth(6)
                        .style(move |_| theme::progress_style(theme::palette::PRIMARY)),
                ]
                .spacing(8)
                .into(),
            )
        }
        UpdateStatus::ReadyToInstall => {
            let version = updater
                .downloaded
                .as_ref()
                .map_or_else(|| "v".to_owned(), |package| format!("v{}", package.version));
            let install_label = if cfg!(target_os = "macos") {
                "打开安装包"
            } else {
                "安装并重启"
            };
            Some(
                row![
                    text(format!("{version} 已准备好"))
                        .size(theme::typography::BODY)
                        .color(theme::palette::TEXT_PRIMARY),
                    row![].width(Fill),
                    button(install_label)
                        .on_press(Message::InstallUpdate)
                        .style(button::primary)
                        .padding([8, 16]),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .into(),
            )
        }
        UpdateStatus::Error => updater
            .error
            .as_ref()
            .map(|error| settings::notice(settings::NoticeKind::Error, &error.user)),
        _ => None,
    }
}

/// Relative "last checked" label from an epoch-milliseconds timestamp.
fn last_checked_text(timestamp: Option<i64>) -> String {
    let Some(millis) = timestamp else {
        return "从未".to_owned();
    };
    let seconds = (chrono::Utc::now().timestamp_millis() - millis).max(0) / 1000;
    match seconds {
        0..=59 => "刚刚".to_owned(),
        60..=3_599 => format!("{} 分钟前", seconds / 60),
        3_600..=86_399 => format!("{} 小时前", seconds / 3_600),
        _ => format!("{} 天前", seconds / 86_400),
    }
}
