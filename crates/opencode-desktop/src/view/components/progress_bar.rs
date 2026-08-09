use crate::message::Message;
use crate::theme;
use iced::widget::progress_bar;
use iced::{Color, Element, Fill};

pub fn view(percent: f64, color: Color, girth: f32) -> Element<'static, Message> {
    progress_bar(0.0..=100.0, percent.clamp(0.0, 100.0) as f32)
        .length(Fill)
        .girth(girth)
        .style(move |_| theme::progress_style(color))
        .into()
}
