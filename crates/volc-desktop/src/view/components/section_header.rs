use crate::message::Message;
use crate::theme;
use iced::widget::{row, text};
use iced::{Element, Fill};

pub fn view(title: &'static str, trailing: Element<'static, Message>) -> Element<'static, Message> {
    row![
        text(title)
            .size(theme::typography::SECTION_TITLE)
            .color(theme::palette::TEXT_PRIMARY)
            .width(Fill),
        trailing,
    ]
    .spacing(theme::spacing::MD)
    .align_y(iced::Alignment::Center)
    .into()
}
