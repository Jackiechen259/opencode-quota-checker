use crate::config::AppConfig;
use crate::message::{Message, ThresholdField};
use crate::state::SettingsState;
use crate::theme;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Fill};

/// Renders monitor configuration and start/stop controls.
pub fn view<'a>(state: &'a SettingsState, config: &AppConfig) -> Element<'a, Message> {
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

    let mut content = column![
        row![
            text("监控设置")
                .size(24)
                .color(theme::palette::TEXT_PRIMARY),
            button("关闭")
                .on_press(Message::CloseSettings)
                .style(theme::soft_button)
                .padding([8, 16]),
        ]
        .spacing(16)
        .align_y(iced::Alignment::Center),
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
    .spacing(16);

    if let Some(error) = &state.error {
        content = content.push(notice_box(error.user.as_str(), true));
    }
    if let Some(notice) = &state.notice {
        content = content.push(notice_box(notice.as_str(), false));
    }
    content.into()
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
