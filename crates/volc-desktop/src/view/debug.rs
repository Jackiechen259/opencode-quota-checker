use crate::message::Message;
use crate::state::UsageState;
use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Fill, Font};

/// Renders the non-persisted raw-response debug overlay.
pub fn view(state: &UsageState) -> Element<'_, Message> {
    let copy = if state.raw.is_some() {
        button("复制 JSON").on_press(Message::CopyRaw)
    } else {
        button("复制 JSON")
    };
    let header = row![
        text("原始 API JSON").size(25).width(Fill),
        copy,
        button("关闭（Esc）").on_press(Message::CloseOverlay)
    ]
    .spacing(10);
    let body: Element<'_, Message> = if state.raw_loading {
        text("正在读取原始响应…").into()
    } else if let Some(raw) = &state.raw {
        scrollable(text(raw).font(Font::MONOSPACE).size(13))
            .height(Fill)
            .into()
    } else if let Some(error) = &state.error {
        column![text(&error.user), text(&error.detail).size(12)]
            .spacing(6)
            .into()
    } else {
        text("暂无原始响应。").into()
    };
    column![header, body].spacing(14).into()
}
