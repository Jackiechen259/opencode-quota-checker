//! Shared hairlines used between metric columns.

use crate::message::Message;
use crate::theme;
use iced::widget::{container, row};
use iced::{Background, Element, Length};

/// A 1px vertical hairline of the given height.
pub fn vertical(height: f32) -> Element<'static, Message> {
    container(row![])
        .width(Length::Fixed(1.0))
        .height(Length::Fixed(height))
        .style(move |_| container::Style {
            background: Some(Background::Color(theme::palette::DIVIDER)),
            ..container::Style::default()
        })
        .into()
}
