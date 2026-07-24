//! A modal confirmation dialog rendered as a stack overlay.

use crate::message::Message;
use crate::theme;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Fill};

/// Renders the confirm dialog as a full-size modal layer (backdrop + card).
/// Render this on top of the page via a `stack` when `open` is true.
pub fn view(open: bool) -> Option<Element<'static, Message>> {
    if !open {
        return None;
    }
    let backdrop = container(row![])
        .width(Fill)
        .height(Fill)
        .style(move |_| theme::backdrop());

    let cancel = button("取消")
        .on_press(Message::CancelClearCredentials)
        .style(theme::soft_button)
        .padding([10, 20]);
    let confirm = button("删除凭证")
        .on_press(Message::ConfirmClearCredentials)
        .style(button::danger)
        .padding([10, 20]);

    let card = container(
        column![
            text("删除访问凭证？")
                .size(20)
                .color(theme::palette::TEXT_PRIMARY),
            text("删除后将无法继续获取方舟配额数据，需要重新配置访问凭证才能恢复。")
                .size(14)
                .color(theme::palette::TEXT_SECONDARY),
            container(row![cancel, confirm].spacing(12),)
                .width(Fill)
                .align_x(Horizontal::Right),
        ]
        .spacing(16)
        .align_x(Horizontal::Left),
    )
    .max_width(440)
    .width(Fill)
    .padding(28)
    .style(move |_| theme::dialog_surface());

    let centered = container(card)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_| theme::backdrop());

    Some(
        iced::widget::stack![backdrop, centered]
            .width(Fill)
            .height(Fill)
            .into(),
    )
}
