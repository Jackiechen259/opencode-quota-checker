//! A drawn status dot, replacing `"●"` glyphs whose baseline never aligns
//! with adjacent text across fonts.

use crate::message::Message;
use crate::theme;
use iced::widget::{container, row};
use iced::{Background, Color, Element, Length};

/// A fixed-size rounded dot in the given color.
pub fn view(color: Color, diameter: f32) -> Element<'static, Message> {
    container(row![])
        .width(Length::Fixed(diameter))
        .height(Length::Fixed(diameter))
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            border: iced::border::rounded(theme::radius::PILL),
            ..container::Style::default()
        })
        .into()
}
