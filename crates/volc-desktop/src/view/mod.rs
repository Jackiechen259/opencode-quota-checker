pub mod app_header;
mod components;
pub mod credentials;
pub mod dashboard;
pub mod debug;
pub mod float;
pub mod format;
pub mod settings;

use crate::message::Message;
use crate::theme;
use crate::view::components::confirm_dialog;
use crate::App;
use iced::widget::{column, container, scrollable, stack, text};
use iced::{Element, Fill};

pub fn main(app: &App) -> Element<'_, Message> {
    let header = app_header::view(
        app.usage().report.as_ref(),
        app.usage().now_ms,
        app.usage().loading,
        app.tray_error(),
    );

    let body: Element<'_, Message> = if app.ui().debug_open {
        debug::view(app.usage())
    } else if app.settings().open {
        settings::view(app.settings(), app.config())
    } else if app.credentials().checking || !app.config_loaded() {
        checking_state()
    } else if !app.credentials().configured {
        credentials::view(app.credentials())
    } else {
        dashboard::view(app.usage())
    };

    let mut content = column![header, body].spacing(0);
    if let Some(toast) = &app.ui().toast {
        content = content.push(
            container(text(toast).color(theme::palette::TEXT_PRIMARY))
                .padding([10, 14])
                .style(move |_| theme::toast()),
        );
    }

    let mut layers = stack![container(scrollable(content))
        .width(Fill)
        .height(Fill)
        .style(move |_| theme::page_background())];

    if let Some(dialog) = confirm_dialog::view(app.confirm_clear_credentials()) {
        layers = layers.push(dialog);
    }

    layers.width(Fill).height(Fill).into()
}

/// Credential-checking placeholder.
fn checking_state() -> Element<'static, Message> {
    container(
        text("正在检查系统钥匙串…")
            .size(15)
            .color(theme::palette::TEXT_MUTED),
    )
    .width(Fill)
    .padding(48)
    .style(move |_| theme::page_background())
    .into()
}

/// Renders the independent floating window from shared state.
pub fn floating(app: &App) -> Element<'_, Message> {
    float::view(app.usage(), app.config().float_mode)
}
