use crate::config::AppConfig;
use crate::message::{Message, ThresholdField};
use crate::state::SettingsState;
use iced::widget::{button, column, row, text, text_input};
use iced::Element;

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
    .spacing(12);
    let monitor_button = if state.saving {
        button("保存中…")
    } else if config.monitor_enabled {
        button("停止监控").on_press(Message::StopMonitor)
    } else {
        button("保存并启动监控").on_press(Message::StartMonitor)
    };
    let mut content = column![
        row![
            text("监控设置").size(26),
            button("关闭").on_press(Message::CloseSettings)
        ]
        .spacing(16),
        text(if config.monitor_enabled {
            "状态：监控运行中"
        } else {
            "状态：监控已停止"
        }),
        fields,
        monitor_button,
    ]
    .spacing(16);
    if let Some(error) = &state.error {
        content = content.push(text(&error.user));
    }
    if let Some(notice) = &state.notice {
        content = content.push(text(notice));
    }
    content.into()
}

fn field<'a>(
    label: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    column![
        text(label).size(13),
        text_input("", value).on_input(on_input).padding(10)
    ]
    .spacing(5)
    .into()
}
