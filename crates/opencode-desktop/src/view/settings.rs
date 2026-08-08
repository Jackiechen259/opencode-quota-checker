use crate::config::AppConfig;
use crate::message::{Message, SensitiveInput, ThresholdField};
use crate::state::{CredentialState, SettingsState};
use crate::theme;
use crate::view::components::{icon_button, icons, settings, status_badge};
use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Element, Fill, Length};

/// Settings page: manage the OpenCode connection and quota-monitoring rules.
///
/// The page is a centered, fixed-width column so it reads as a focused
/// settings dialog rather than a full-bleed form.
pub fn view<'a>(
    state: &'a SettingsState,
    config: &AppConfig,
    credentials: &'a CredentialState,
) -> Element<'a, Message> {
    let mut children: Vec<Element<'a, Message>> = vec![
        header(),
        account_card(credentials),
        monitor_card(state, config),
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
