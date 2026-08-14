//! Application-drawn title bar of the borderless main window.
//!
//! Layout, from left to right: app icon, app name, a draggable spacer, then
//! the minimize / maximize-restore / close window controls. The drag region
//! and the controls are separate sibling areas so button clicks can never be
//! swallowed by the drag handler.
//!
//! Dimensions track a native Windows caption: a 32 px strip with 46 px wide
//! square-hit caption buttons and one shared 11 px glyph box per button.

use crate::message::Message;
use crate::theme;
use crate::view::components::{app_icon, icons};
use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::svg;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{mouse, Layout};
use iced::widget::tooltip::Position;
use iced::widget::{button, container, mouse_area, row, text, tooltip, Space};
use iced::{alignment, Alignment, Element, Length, Padding, Radians, Rectangle, Size};

/// Height of the title bar in logical pixels.
pub const HEIGHT: f32 = 32.0;
/// Width of each window-control button (the full hit target).
const CONTROL_WIDTH: f32 = 46.0;
/// Side length of the app icon in the brand area.
const APP_ICON_SIZE: f32 = 16.0;
/// Font size of the app title.
const TITLE_FONT_SIZE: f32 = 13.0;
/// Side length of the shared caption-glyph box in every window control.
const CONTROL_ICON_SIZE: f32 = 11.0;
/// Left padding of the brand area.
const BRAND_LEFT_PADDING: f32 = 10.0;
/// Right padding of the brand area.
const BRAND_RIGHT_PADDING: f32 = 12.0;
/// Spacing between the app icon and the title.
const BRAND_SPACING: f32 = 7.0;

pub fn view(maximized: Option<bool>) -> Element<'static, Message> {
    let brand = row![
        app_icon::view(APP_ICON_SIZE),
        text("OpenCode Quota Checker")
            .size(TITLE_FONT_SIZE)
            .color(theme::palette::TEXT_PRIMARY),
    ]
    .spacing(BRAND_SPACING)
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 0.0,
        right: BRAND_RIGHT_PADDING,
        bottom: 0.0,
        left: BRAND_LEFT_PADDING,
    });

    // Everything left of the window controls is one drag region; double
    // clicking it toggles maximize / restore like a native caption bar. The
    // brand is centered vertically (containers default to top alignment).
    let drag_region = mouse_area(
        container(row![brand, Space::new().width(Length::Fill)])
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(alignment::Vertical::Center),
    )
    .on_press(Message::DragMainWindow)
    .on_double_click(Message::ToggleMaximizeMainWindow);

    let controls = row![
        window_control(
            Message::MinimizeMainWindow,
            "最小化",
            icons::WIN_MINIMIZE,
            false
        ),
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
            false,
        ),
        window_control(Message::CloseMainWindow, "关闭", icons::WIN_CLOSE, true),
    ];

    container(row![drag_region, controls])
        .width(Length::Fill)
        .height(Length::Fixed(HEIGHT))
        .style(|_| theme::title_bar_surface())
        .into()
}

/// A square, borderless caption button. Every window control — minimize,
/// maximize-restore and close — goes through this one helper, so all three
/// share the same width, height, glyph box and centering; only the style
/// differs (close gets the Windows red hover).
fn window_control(
    message: Message,
    label: &'static str,
    glyph: &'static [u8],
    close: bool,
) -> Element<'static, Message> {
    // Buttons place their content at the padding origin, so the glyph is
    // centered with a fill container instead of manual offsets.
    let glyph = container(caption_glyph(glyph))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);

    let button = button(glyph)
        .width(Length::Fixed(CONTROL_WIDTH))
        .height(Length::Fixed(HEIGHT))
        .padding(Padding::ZERO)
        .on_press(message)
        .style(if close {
            theme::title_bar_close_button
        } else {
            theme::title_bar_button
        });

    with_tooltip(button, label)
}

/// The caption glyph: a fixed-size SVG box that, like `Text`, inherits the
/// `text_color` of the enclosing widget. This is what lets the close button
/// flip its icon to white on hover without duplicating the widget tree or
/// hardcoding a color in `svg::Style`.
fn caption_glyph(bytes: &'static [u8]) -> Element<'static, Message> {
    Element::new(CaptionGlyph {
        handle: icons::handle(bytes),
        size: CONTROL_ICON_SIZE,
    })
}

/// Renders an SVG glyph stretched into a fixed square box, recolored with the
/// inherited `renderer::Style::text_color` (the same value `Text` falls back
/// to), so caption icons follow the button's hover / pressed appearance.
struct CaptionGlyph {
    handle: svg::Handle,
    size: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for CaptionGlyph
where
    Renderer: svg::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.size),
            height: Length::Fixed(self.size),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(
            Length::Fixed(self.size),
            Length::Fixed(self.size),
            Size::new(self.size, self.size),
        ))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        renderer.draw_svg(
            svg::Svg {
                handle: self.handle.clone(),
                color: Some(style.text_color),
                rotation: Radians(0.0),
                opacity: 1.0,
            },
            layout.bounds(),
            layout.bounds(),
        );
    }
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
