use crate::message::Message;
use crate::state::UsageState;
use crate::theme;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill, Font};

/// Renders the non-persisted raw-response debug overlay.
pub fn view(state: &UsageState) -> Element<'_, Message> {
    let copy = if state.raw.is_some() {
        button("复制 JSON")
            .on_press(Message::CopyRaw)
            .style(button::primary)
            .padding([8, 16])
    } else {
        button("复制 JSON").padding([8, 16])
    };
    let header = row![
        text("原始 API JSON")
            .size(22)
            .width(Fill)
            .color(theme::palette::TEXT_PRIMARY),
        copy,
        button("关闭（Esc）")
            .on_press(Message::CloseOverlay)
            .style(theme::soft_button)
            .padding([8, 16]),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);
    let body: Element<'_, Message> = if state.raw_loading {
        text("正在读取原始响应…")
            .color(theme::palette::TEXT_MUTED)
            .into()
    } else if let Some(raw) = &state.raw {
        scrollable(
            text(raw)
                .font(Font::MONOSPACE)
                .size(13)
                .color(theme::palette::TEXT_PRIMARY),
        )
        .height(Fill)
        .into()
    } else if let Some(error) = &state.error {
        container(
            column![
                text(&error.user).color(theme::palette::TEXT_PRIMARY),
                text(&error.detail)
                    .size(12)
                    .color(theme::palette::TEXT_MUTED),
            ]
            .spacing(6),
        )
        .width(Fill)
        .padding(14)
        .style(move |_| theme::danger_box())
        .into()
    } else {
        text("暂无原始响应。")
            .color(theme::palette::TEXT_MUTED)
            .into()
    };
    column![header, body].spacing(14).into()
}
