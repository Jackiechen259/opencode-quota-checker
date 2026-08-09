use crate::message::Message;
use crate::theme;
use crate::view::components::settings::icon_tile;
use iced::widget::{column, row, text};
use iced::{Element, Fill};

/// Section title with the settings-style icon brick, a one-line description
/// and a right-aligned trailing element.
pub fn with_icon(
    icon: &'static [u8],
    title: &'static str,
    description: &'static str,
    trailing: Element<'static, Message>,
) -> Element<'static, Message> {
    row![
        icon_tile(icon),
        column![
            text(title)
                .size(theme::typography::SECTION_TITLE)
                .color(theme::palette::TEXT_PRIMARY),
            text(description)
                .size(theme::typography::CAPTION)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(2),
        row![].width(Fill),
        trailing,
    ]
    .spacing(theme::spacing::MD)
    .align_y(iced::Alignment::Center)
    .into()
}
