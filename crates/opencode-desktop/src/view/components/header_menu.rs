//! Overflow menu of the control bar, rendered as a stack overlay.
//!
//! The overlay is fully transparent (no backdrop dimming): it only swallows
//! clicks outside the card so the menu closes like a native dropdown.

use crate::message::{HeaderAction, Message};
use crate::theme;
use crate::view::app_header;
use crate::view::components::icons;
use crate::view::title_bar;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, mouse_area, row, rule, svg, text};
use iced::{Element, Fill, Length};

/// Renders the header overflow menu as a full-size overlay layer. Render this
/// on top of the page via a `stack` when `open` is true.
pub fn view(open: bool, focused: Option<HeaderAction>) -> Option<Element<'static, Message>> {
    if !open {
        return None;
    }
    let card = container(
        column![
            menu_item(
                icons::CODE,
                "开发者详情",
                HeaderAction::Details,
                focused,
                false
            ),
            menu_item(
                icons::HIDE,
                "隐藏到托盘",
                HeaderAction::Hide,
                focused,
                false
            ),
            rule::horizontal(1).style(menu_rule),
            menu_item(icons::EXIT, "退出", HeaderAction::Exit, focused, true),
        ]
        .spacing(2),
    )
    .width(Length::Fixed(200.0))
    .padding(6)
    .style(move |_| theme::menu_surface());

    // The card's top-right corner lands just below the ⋯ button: the two
    // heights are constants, so tuning a bar height stays in sync.
    let positioned = container(card)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Top)
        .padding([
            title_bar::HEIGHT + app_header::HEIGHT + 4.0,
            theme::spacing::PAGE_PADDING,
        ]);

    Some(
        iced::widget::stack![
            mouse_area(container(row![]).width(Fill).height(Fill))
                .on_press(Message::CloseHeaderMenu),
            positioned,
        ]
        .width(Fill)
        .height(Fill)
        .into(),
    )
}

/// One menu item; `danger` selects the red hover tint used by 退出.
fn menu_item(
    icon: &'static [u8],
    label: &'static str,
    action: HeaderAction,
    focused: Option<HeaderAction>,
    danger: bool,
) -> Element<'static, Message> {
    let is_focused = focused == Some(action);
    let icon_color = if is_focused {
        if danger {
            theme::palette::DANGER
        } else {
            theme::palette::PRIMARY
        }
    } else {
        theme::palette::TEXT_SECONDARY
    };
    let style = if danger {
        theme::menu_item_danger as fn(&iced::Theme, button::Status, bool) -> button::Style
    } else {
        theme::menu_item as fn(&iced::Theme, button::Status, bool) -> button::Style
    };

    button(
        row![
            svg(icons::handle(icon))
                .style(move |_theme, _status| svg::Style {
                    color: Some(icon_color),
                })
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0)),
            text(label).size(theme::typography::BODY),
        ]
        .spacing(theme::spacing::SM)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .padding([8, 10])
    .on_press(Message::HeaderPressed(action))
    .style(move |theme, status| style(theme, status, is_focused))
    .into()
}

fn menu_rule(_theme: &iced::Theme) -> rule::Style {
    rule::Style {
        color: theme::palette::DIVIDER,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: false,
    }
}
