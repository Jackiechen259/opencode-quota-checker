use crate::message::Message;
use crate::theme;
use iced::widget::{column, text};
use iced::{Element, Fill, Font};

pub fn value(label: &'static str, value: String) -> Element<'static, Message> {
    column![
        text(label)
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_MUTED),
        text(value)
            .font(Font::MONOSPACE)
            .size(theme::typography::METRIC_VALUE)
            .color(theme::palette::TEXT_PRIMARY),
    ]
    .spacing(theme::spacing::XS)
    .width(Fill)
    .into()
}
