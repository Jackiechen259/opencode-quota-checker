mod components;
mod credentials;
mod dashboard;
mod debug;
mod float;
mod format;
mod settings;

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
        button("设置").on_press(Message::OpenSettings),
        button("悬浮窗").on_press(Message::ToggleFloat),
        button("隐藏").on_press(Message::HideMain),
        button("退出").on_press(Message::Exit),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);
    let body: Element<'_, Message> = if app.ui().debug_open {
        debug::view(app.usage())
    } else if app.settings().open {
        settings::view(app.settings(), app.config())
    } else if app.credentials().checking || !app.config_loaded() {
        text("正在检查系统钥匙串…").into()
    } else if !app.credentials().configured {
        credentials::view(app.credentials())
    } else {
        dashboard::view(app.usage())
    };
    let mut content = column![header, body].spacing(20).padding(28);
    if let Some(toast) = &app.ui().toast {
        content = content.push(
            container(text(toast))
                .padding(10)
                .style(container::rounded_box),
        );
    }

    container(scrollable(content))
        .width(Fill)
        .height(Fill)
        .into()
}

/// Renders the independent floating window from shared state.
pub fn floating(app: &App) -> Element<'_, Message> {
    float::view(app.usage(), app.config().float_mode)
}
