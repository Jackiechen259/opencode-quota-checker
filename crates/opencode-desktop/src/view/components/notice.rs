//! Inline feedback strips shared across pages (success / error / warning).

use crate::message::Message;
use crate::theme;
use iced::widget::{container, row, text};
use iced::{Element, Fill};

/// Tone of an inline notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    /// Positive confirmation.
    Success,
    /// Something went wrong.
    Error,
    /// A heads-up that is not fatal.
    Warning,
}

/// Inline feedback strip, not a third card.
pub fn view(kind: NoticeKind, message: &str) -> Element<'static, Message> {
    let mark_color = match kind {
        NoticeKind::Success => theme::palette::SUCCESS,
        NoticeKind::Error => theme::palette::DANGER,
        NoticeKind::Warning => theme::palette::WARNING,
    };
    let mark = match kind {
        NoticeKind::Success => "✓",
        NoticeKind::Error | NoticeKind::Warning => "!",
    };
    container(
        row![
            text(mark).size(14).color(mark_color),
            text(message.to_owned())
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_PRIMARY),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .padding([10, 14])
    .style(move |_| match kind {
        NoticeKind::Success => theme::success_box(),
        NoticeKind::Error => theme::danger_box(),
        NoticeKind::Warning => theme::warning_box(),
    })
    .into()
}
