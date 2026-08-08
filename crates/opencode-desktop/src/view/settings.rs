use crate::config::AppConfig;
use crate::message::{Message, SensitiveInput, ThresholdField};
use crate::state::{CredentialState, SettingsState};
use crate::theme;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Fill};

/// Renders the OpenCode Go data source, monitor configuration and start/stop controls.
pub fn view<'a>(
    state: &'a SettingsState,
    config: &AppConfig,
    credentials: &'a CredentialState,
) -> Element<'a, Message> {
    let mut content = column![
        header(),
        opencode_section(credentials),
        monitor_section(state, config)
    ]
    .spacing(16);

    if let Some(error) = &state.error {
        content = content.push(notice_box(error.user.as_str(), true));
    }
    if let Some(notice) = &state.notice {
        content = content.push(notice_box(notice.as_str(), false));
    }
    content.into()
}

fn header() -> Element<'static, Message> {
    row![
        text("设置").size(24).color(theme::palette::TEXT_PRIMARY),
        text("数据源：OpenCode Go")
            .size(13)
            .color(theme::palette::TEXT_MUTED),
        row![].width(Fill),
        button("关闭")
            .on_press(Message::CloseSettings)
            .style(theme::soft_button)
            .padding([8, 16]),
    ]
    .spacing(16)
    .align_y(iced::Alignment::Center)
    .into()
}

/// Editable OpenCode Go data source (workspace ID + auth cookie).
fn opencode_section<'a>(state: &'a CredentialState) -> Element<'a, Message> {
    let workspace = field(
        "Workspace ID",
        &state.opencode_workspace,
        Message::OpenCodeWorkspaceChanged,
    );
    let cookie = column![
        text("Auth Cookie")
            .size(12)
            .color(theme::palette::TEXT_MUTED),
        text_input("", &state.opencode_cookie)
            .on_input(|value| Message::OpenCodeCookieChanged(SensitiveInput(value)))
            .secure(true)
            .padding(10),
        text("请将 Auth Cookie 视为密码保管；它仅保存在系统钥匙串，随请求发送到 opencode.ai。")
            .size(11)
            .color(theme::palette::WARNING),
    ]
    .spacing(5);

    let can_save = !state.mutating
        && !state.opencode_workspace.trim().is_empty()
        && !state.opencode_cookie.trim().is_empty();
    let save = if can_save {
        button("保存 OpenCode 配置")
            .on_press(Message::SaveOpenCodeCredentials)
            .style(button::primary)
            .padding([10, 20])
    } else {
        button("保存 OpenCode 配置").padding([10, 20])
    };

    container(
        column![
            text("OpenCode Go 数据源")
                .size(16)
                .color(theme::palette::TEXT_PRIMARY),
            workspace,
            cookie,
            save,
        ]
        .spacing(14),
    )
    .width(Fill)
    .padding(18)
    .style(move |_| theme::panel())
    .into()
}

fn monitor_section<'a>(state: &'a SettingsState, config: &AppConfig) -> Element<'a, Message> {
    let fields = column![
        field(
            "轮询间隔（30–3600 秒）",
            &state.interval,
            Message::IntervalChanged
        ),
        field("5 小时阈值（0–100%）", &state.five_hour, |value| {
            Message::ThresholdChanged(ThresholdField::FiveHour, value)
        }),
        field("近一周阈值（0–100%）", &state.weekly, |value| {
            Message::ThresholdChanged(ThresholdField::Weekly, value)
        }),
        field("近一月阈值（0–100%）", &state.monthly, |value| {
            Message::ThresholdChanged(ThresholdField::Monthly, value)
        }),
    ]
    .spacing(14);

    let monitor_button = if state.saving {
        button("保存中…").padding([10, 20])
    } else if config.monitor_enabled {
        button("停止监控")
            .on_press(Message::StopMonitor)
            .style(button::danger)
            .padding([10, 20])
    } else {
        button("保存并启动监控")
            .on_press(Message::StartMonitor)
            .style(button::primary)
            .padding([10, 20])
    };

    let status_label = if config.monitor_enabled {
        "状态：监控运行中"
    } else {
        "状态：监控已停止"
    };

    column![
        text(status_label)
            .size(13)
            .color(theme::palette::TEXT_MUTED),
        container(fields)
            .width(Fill)
            .padding(18)
            .style(move |_| theme::panel()),
        monitor_button,
        button("删除访问凭证")
            .on_press(Message::ClearCredentials)
            .style(button::danger)
            .padding([10, 20]),
    ]
    .spacing(16)
    .into()
}

fn field<'a>(
    label: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    column![
        text(label).size(12).color(theme::palette::TEXT_MUTED),
        text_input("", value).on_input(on_input).padding(10),
    ]
    .spacing(5)
    .into()
}

/// A small notice row: danger-tinted for errors, neutral panel otherwise.
fn notice_box(message: &str, is_error: bool) -> Element<'_, Message> {
    container(text(message.to_owned()).color(theme::palette::TEXT_PRIMARY))
        .width(Fill)
        .padding([10, 14])
        .style(move |_| {
            if is_error {
                theme::danger_box()
            } else {
                theme::panel()
            }
        })
        .into()
}
