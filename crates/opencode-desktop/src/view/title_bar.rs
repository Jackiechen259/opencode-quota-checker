//! Application-drawn title bar of the borderless main window.
//!
//! Layout, from left to right: app icon, app name, a draggable spacer, then
//! the minimize / maximize-restore / close window controls. The drag region
//! and the controls are separate sibling areas so button clicks can never be
//! swallowed by the drag handler.

use crate::message::Message;
use crate::theme;
use crate::view::components::{app_icon, icons};
use iced::widget::tooltip::Position;
use iced::widget::{button, container, mouse_area, row, svg, text, tooltip, Space};
use iced::{Alignment, Element, Length, Padding};

/// Height of the title bar in logical pixels.
pub const HEIGHT: f32 = 44.0;
/// Width of each window-control button.
const CONTROL_WIDTH: f32 = 44.0;

pub fn view(maximized: Option<bool>) -> Element<'static, Message> {
    let brand = row![
        app_icon::view(20.0),
        text("OpenCode Quota Checker")
            .size(14.0)
            .color(theme::palette::TEXT_PRIMARY),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 0.0,
        right: 16.0,
        bottom: 0.0,
        left: 12.0,
    });

    // Everything left of the window controls is one drag region; double
    // clicking it toggles maximize / restore like a native caption bar.
    let drag_region = mouse_area(
        container(row![brand, Space::new().width(Length::Fill)])
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::DragMainWindow)
    .on_double_click(Message::ToggleMaximizeMainWindow);

    let controls = row![
        window_control(Message::MinimizeMainWindow, "最小化", icons::WIN_MINIMIZE,),
        window_control(
            Message::ToggleMaximizeMainWindow,
            if maximized == Some(true) {
                "还原"
            } else {
                "最大化"
            },
            if maximized == Some(true) {
                icons::WIN_RESTORE
            } else {
                icons::WIN_MAXIMIZE
            },
        ),
        close_control(),
    ];

    container(row![drag_region, controls])
        .width(Length::Fill)
        .height(Length::Fixed(HEIGHT))
        .style(|_| theme::title_bar_surface())
        .into()
}

/// A square, borderless window-control button with an SVG glyph.
fn window_control(
    message: Message,
    label: &'static str,
    glyph: &'static [u8],
) -> Element<'static, Message> {
    let button = button(
        svg(icons::handle(glyph))
            .style(|_theme, _status| iced::widget::svg::Style {
                color: Some(theme::palette::TEXT_SECONDARY),
            })
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0)),
    )
    .width(Length::Fixed(CONTROL_WIDTH))
    .height(Length::Fixed(HEIGHT))
    .padding(Padding::ZERO)
    .on_press(message)
    .style(theme::title_bar_button);

    with_tooltip(button, label)
}

/// Close button: a plain `×` glyph that inherits the button text color, so
/// the hover state flips it to white on the Windows-style red background.
fn close_control() -> Element<'static, Message> {
    let button = button(text("×").size(20.0))
        .width(Length::Fixed(CONTROL_WIDTH))
        .height(Length::Fixed(HEIGHT))
        .padding(Padding::ZERO)
        .on_press(Message::CloseMainWindow)
        .style(theme::title_bar_close_button);

    with_tooltip(button, "关闭")
}

fn with_tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'static str,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(
            text(label)
                .size(theme::typography::LABEL)
                .color(theme::palette::SURFACE),
        )
        .padding([6, 10])
        .style(|_| theme::tooltip_surface()),
        Position::Bottom,
    )
    .gap(6)
    .into()
}
