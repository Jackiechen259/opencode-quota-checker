use crate::message::Message;
use crate::theme;
use iced::widget::{container, text};
use iced::{Background, Border, Color, Element};

#[derive(Debug, Clone, Copy)]
pub enum Tone {
    Primary,
    Success,
    Warning,
    Danger,
    Neutral,
}

impl Tone {
    fn colors(self) -> (Color, Color, Color) {
        match self {
            Self::Primary => (
                theme::palette::PRIMARY,
                theme::palette::PRIMARY_LIGHT,
                theme::palette::PRIMARY_BORDER,
            ),
            Self::Success => (
                theme::palette::SUCCESS,
                theme::palette::SUCCESS_LIGHT,
                theme::palette::SUCCESS_BORDER,
            ),
            Self::Warning => (
                theme::palette::WARNING,
                theme::palette::WARNING_LIGHT,
                theme::palette::WARNING_BORDER,
            ),
            Self::Danger => (
                theme::palette::DANGER,
                theme::palette::DANGER_LIGHT,
                theme::palette::DANGER_BORDER,
            ),
            Self::Neutral => (
                theme::palette::TEXT_SECONDARY,
                theme::palette::SURFACE_HOVER,
                theme::palette::BORDER,
            ),
        }
    }
}

pub fn view(label: impl Into<String>, tone: Tone) -> Element<'static, Message> {
    let (foreground, background, border) = tone.colors();
    container(
        text(label.into())
            .size(theme::typography::CAPTION)
            .color(foreground),
    )
    .padding([4, 9])
    .style(move |_| container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            color: border,
            width: 1.0,
            radius: theme::radius::PILL.into(),
        },
        ..container::Style::default()
    })
    .into()
}
