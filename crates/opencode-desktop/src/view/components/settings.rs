//! Reusable building blocks for the settings page.

use crate::message::Message;
use crate::theme;
use crate::view::components::icons;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::svg;
use iced::widget::{button, column, container, row, text, text_input, Container};
use iced::{Element, Fill, FillPortion, Length};

/// Re-exported inline notice so settings call sites keep `settings::notice`.
pub use super::notice::{view as notice, NoticeKind};

/// Card surface shared by the settings cards: white, hairline border,
/// light shadow and a consistent 20px padding.
pub fn settings_card<'a>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content)
        .width(Fill)
        .padding(theme::spacing::CARD_PADDING)
        .style(move |_| theme::card())
}

/// Icon tile + title + description header row with an optional trailing
/// element (e.g. the monitor status badge).
pub fn card_header(
    icon: &'static [u8],
    title: &'static str,
    description: &'static str,
    trailing: Option<Element<'static, Message>>,
) -> Element<'static, Message> {
    let mut header = row![
        icon_tile(icon),
        column![
            text(title)
                .size(theme::typography::CARD_TITLE)
                .color(theme::palette::TEXT_PRIMARY),
            text(description)
                .size(theme::typography::LABEL)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(2),
        row![].width(Fill),
    ]
    .spacing(theme::spacing::MD)
    .align_y(iced::Alignment::Center);
    if let Some(trailing) = trailing {
        header = header.push(trailing);
    }
    header.into()
}

/// A labeled form input with an optional helper line beneath it.
pub fn form_field<'a>(
    label: &'static str,
    input: Element<'a, Message>,
    helper: Option<&'a str>,
) -> Element<'a, Message> {
    let mut content = column![
        text(label)
            .size(theme::typography::BODY)
            .color(theme::palette::TEXT_SECONDARY),
        input,
    ]
    .spacing(6)
    .width(Fill);
    if let Some(helper) = helper {
        content = content.push(
            text(helper)
                .size(theme::typography::CAPTION)
                .color(theme::palette::TEXT_MUTED),
        );
    }
    content.into()
}

/// A compact threshold input: label above, numeric input with a `%` suffix.
pub fn threshold_field<'a>(
    label: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    column![
        text(label)
            .size(theme::typography::BODY)
            .color(theme::palette::TEXT_SECONDARY),
        row![
            text_input("", value)
                .on_input(on_input)
                .padding([9, 10])
                .style(theme::settings_input)
                .width(Fill),
            text("%")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_SECONDARY),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(6)
    .width(FillPortion(1))
    .into()
}

/// The isolated destructive-actions card.
pub fn danger_zone(can_delete: bool) -> Element<'static, Message> {
    let delete = if can_delete {
        button("删除访问凭证")
            .on_press(Message::ClearCredentials)
            .style(theme::danger_outline_button)
            .padding([8, 16])
    } else {
        button("删除访问凭证")
            .style(theme::danger_outline_button)
            .padding([8, 16])
    };
    container(
        column![
            text("危险操作")
                .size(theme::typography::CARD_TITLE)
                .color(theme::palette::TEXT_PRIMARY),
            text("删除本机保存的 OpenCode 访问凭证")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_MUTED),
            container(delete).width(Fill).align_x(Horizontal::Right),
        ]
        .spacing(theme::spacing::BASE),
    )
    .width(Fill)
    .padding(20)
    .style(move |_| theme::danger_zone())
    .into()
}

/// Blue icon brick used by card headers (shared with the main page's section
/// headers so both pages render the identical tile).
pub fn icon_tile(icon: &'static [u8]) -> Element<'static, Message> {
    container(
        svg(icons::handle(icon))
            .style(move |_theme, _status| svg::Style {
                color: Some(theme::palette::PRIMARY),
            })
            .width(Length::Fixed(18.0))
            .height(Length::Fixed(18.0)),
    )
    .width(Length::Fixed(32.0))
    .height(Length::Fixed(32.0))
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .style(move |_| theme::settings_icon_tile())
    .into()
}
