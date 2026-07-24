use crate::message::Message;
use crate::App;
use iced::widget::{button, column, container, text};
use iced::{Element, Fill};

/// Renders the Phase 3 native application shell.
pub fn main(app: &App) -> Element<'_, Message> {
    let tray_status = app.tray_error().map_or_else(
        || "系统托盘已连接；关闭窗口后仍可从托盘恢复。".to_owned(),
        |error| format!("系统托盘不可用；关闭窗口将退出：{error}"),
    );
    let content = column![
        text("VOLC Status").size(32),
        text("Pure Rust + Iced application skeleton").size(18),
        text(tray_status),
        button("隐藏到托盘").on_press(Message::HideMain),
        button("退出").on_press(Message::Exit),
    ]
    .spacing(18)
    .padding(32);

    container(content)
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}
