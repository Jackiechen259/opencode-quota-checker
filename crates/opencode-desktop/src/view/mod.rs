pub mod app_header;
mod components;
pub mod credentials;
pub mod dashboard;
pub mod debug;
pub mod float;
pub mod footer;
pub mod format;
pub mod overview;
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
        app.ui().header_focus,
        app.config().provider == opencode_core::Provider::VolcArkV,
    );

    let dashboard_open = !app.ui().debug_open
        && !app.settings().open
        && !app.credentials().checking
        && app.config_loaded()
        && app.provider_configured();

    let body: Element<'_, Message> = if app.ui().debug_open {
        debug::view(app.usage())
    } else if app.settings().open {
        settings::view(app.settings(), app.config(), app.credentials())
    } else if app.credentials().checking || !app.config_loaded() {
        checking_state()
    } else if !app.provider_configured() {
        credentials::view(app.credentials(), app.config().provider)
    } else {
        dashboard::view(app.usage())
    };

    let body: Element<'_, Message> = if dashboard_open {
        scrollable(body).width(Fill).height(Fill).into()
    } else {
        container(body).width(Fill).height(Fill).into()
    };

    let content: Element<'_, Message> = if dashboard_open {
        column![
            header,
            body,
            footer::view(
                app.usage().report.as_ref(),
                app.usage().now_ms,
                app.usage().loading
            )
        ]
        .spacing(0)
        .into()
    } else {
        column![header, body].spacing(0).into()
    };

    let mut layers = stack![container(content)
        .width(Fill)
        .height(Fill)
        .style(move |_| theme::page_background())];

    if let Some(toast) = &app.ui().toast {
        layers = layers.push(
            container(
                container(text(toast).color(theme::palette::TEXT_PRIMARY))
                    .padding([10, 14])
                    .style(move |_| theme::toast()),
            )
            .width(Fill)
            .height(Fill)
            .padding(theme::spacing::PAGE_PADDING)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Bottom),
        );
    }

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
    float::view(app.usage(), app.float_mode())
}
