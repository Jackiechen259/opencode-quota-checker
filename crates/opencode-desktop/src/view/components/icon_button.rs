use crate::message::Message;
use crate::theme;
use crate::view::components::{icons, loading_spinner::LoadingSpinner};
use iced::widget::tooltip::Position;
use iced::widget::{button, canvas, container, svg, text, tooltip};
use iced::{Element, Length};

pub fn view(
    icon: &'static [u8],
    label: &'static str,
    message: Message,
    focused: bool,
) -> Element<'static, Message> {
    let icon_color = if focused {
        theme::palette::PRIMARY
    } else {
        theme::palette::TEXT_SECONDARY
    };
    let content = svg(icons::handle(icon))
        .style(move |_theme, _status| svg::Style {
            color: Some(icon_color),
        })
        .width(Length::Fixed(19.0))
        .height(Length::Fixed(19.0));

    with_tooltip(
        button(content)
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .padding(8)
            .on_press(message)
            .style(move |theme, status| theme::icon_button_with_focus(theme, status, focused))
            .into(),
        label,
    )
}

pub fn loading(now_ms: i64, label: &'static str, focused: bool) -> Element<'static, Message> {
    let spinner = canvas(LoadingSpinner::new(now_ms))
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0));
    with_tooltip(
        button(spinner)
            .width(Length::Fixed(36.0))
            .height(Length::Fixed(36.0))
            .padding(8)
            .style(move |theme, status| theme::icon_button_with_focus(theme, status, focused))
            .into(),
        label,
    )
}

fn with_tooltip(
    content: Element<'static, Message>,
    label: &'static str,
) -> Element<'static, Message> {
    tooltip(
        content,
        container(
            text(label)
                .size(theme::typography::LABEL)
                .color(theme::palette::SURFACE),
        )
        .padding([6, 10])
        .style(move |_| theme::tooltip_surface()),
        Position::Bottom,
    )
    .gap(6)
    .into()
}
