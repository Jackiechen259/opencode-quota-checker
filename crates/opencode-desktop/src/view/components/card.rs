use crate::message::Message;
use crate::theme;
use iced::widget::{container, Container};
use iced::{Element, Length};

pub fn standard<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content)
        .width(Length::Fill)
        .padding(theme::spacing::CARD_PADDING)
        .style(move |_| theme::card())
}

pub fn fixed<'a>(content: impl Into<Element<'a, Message>>, height: f32) -> Container<'a, Message> {
    standard(content).height(Length::Fixed(height))
}

pub fn sized<'a>(
    content: impl Into<Element<'a, Message>>,
    width: f32,
    height: f32,
) -> Container<'a, Message> {
    standard(content)
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
}
