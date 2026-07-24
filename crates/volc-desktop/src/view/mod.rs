mod components;
mod credentials;
mod dashboard;
mod format;

use crate::message::Message;
use crate::App;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill};

/// Renders the native application shell and current business state.
pub fn main(app: &App) -> Element<'_, Message> {
    let tray_status = app.tray_error().map_or_else(
        || "托盘在线".to_owned(),
        |error| format!("托盘不可用：{error}"),
    );
    let header = row![
        column![
            text("VOLC Status").size(32),
            text("火山方舟 Agent Plan AFP 配额监控")
        ]
        .spacing(4)
        .width(Fill),
        text(tray_status).size(13),
        button("隐藏").on_press(Message::HideMain),
        button("退出").on_press(Message::Exit),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);
    let body: Element<'_, Message> = if app.credentials().checking {
        text("正在检查系统钥匙串…").into()
    } else if !app.credentials().configured {
        credentials::view(app.credentials())
    } else {
        dashboard::view(app.usage())
    };
    let content = column![header, body].spacing(20).padding(28);

    container(scrollable(content))
        .width(Fill)
        .height(Fill)
        .into()
}
