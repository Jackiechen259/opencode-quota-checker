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
use crate::state::UpdateStatus;
use crate::theme;
use crate::view::components::{confirm_dialog, icon_button, icons};
use crate::App;
use iced::widget::{button, column, container, row, scrollable, stack, text, Row};
use iced::{Element, Fill};

pub fn main(app: &App) -> Element<'_, Message> {
    let header = app_header::view(
        app.usage().report.as_ref(),
        app.usage().now_ms,
        app.usage().loading,
        app.tray_error(),
        app.ui().header_focus,
        true,
    );

    let dashboard_open = !app.ui().debug_open
        && !app.settings().open
        && !app.credentials().checking
        && app.config_loaded()
        && app.configured();

    let body: Element<'_, Message> = if app.ui().debug_open {
        debug::view(app.usage())
    } else if app.settings().open {
        settings::view(
            app.settings(),
            app.config(),
            app.credentials(),
            app.updater(),
        )
    } else if app.credentials().checking || !app.config_loaded() {
        checking_state()
    } else if !app.configured() {
        credentials::view(app.credentials())
    } else {
        dashboard::view(app.usage())
    };

    let body: Element<'_, Message> = if dashboard_open {
        scrollable(body).width(Fill).height(Fill).into()
    } else {
        container(body).width(Fill).height(Fill).into()
    };

    let content: Element<'_, Message> = if dashboard_open {
        let mut items: Vec<Element<'_, Message>> = vec![header];
        if let Some(banner) = update_banner(app) {
            items.push(banner);
        }
        items.push(body);
        items.push(footer::view(
            app.usage().report.as_ref(),
            app.usage().now_ms,
            app.usage().loading,
        ));
        column(items).spacing(0).into()
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

/// Non-blocking notification strip pinned above the dashboard when an update
/// is available. It never interrupts interaction; the user can dismiss it for
/// this run while the available version is kept in settings.
fn update_banner(app: &App) -> Option<Element<'_, Message>> {
    if !app.updater().banner_visible() {
        return None;
    }
    let updater = app.updater();
    let (label, install) = match updater.status {
        UpdateStatus::ReadyToInstall => {
            let version = updater
                .downloaded
                .as_ref()
                .map_or_else(|| "v".to_owned(), |package| format!("v{}", package.version));
            let install_label = if cfg!(target_os = "macos") {
                "打开安装包"
            } else {
                "安装并重启"
            };
            (
                format!("新版本 {version} 已准备好"),
                Some(button(install_label).on_press(Message::InstallUpdate)),
            )
        }
        UpdateStatus::Downloading => {
            let tag = updater
                .available
                .as_ref()
                .map_or_else(|| "更新".to_owned(), |info| info.tag.clone());
            (format!("正在下载 {tag}…"), None)
        }
        _ => {
            let tag = updater
                .available
                .as_ref()
                .map_or_else(|| "更新".to_owned(), |info| info.tag.clone());
            (
                format!("新版本 {tag} 可用"),
                Some(button("更新").on_press(Message::DownloadUpdate)),
            )
        }
    };

    let mut actions: Vec<Element<'_, Message>> = vec![button("查看")
        .on_press(Message::OpenReleaseNotes)
        .style(theme::soft_button)
        .padding([6, 12])
        .into()];
    if let Some(install) = install {
        actions.push(install.style(button::primary).padding([6, 12]).into());
    }
    actions.push(icon_button::view(
        icons::CLOSE,
        "关闭",
        Message::DismissUpdate,
        false,
    ));

    Some(
        container(
            row![
                text("✨").size(14),
                text(label)
                    .size(theme::typography::BODY)
                    .color(theme::palette::TEXT_PRIMARY),
                row![].width(Fill),
                Row::with_children(actions)
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .padding([10, 24])
        .style(move |_| theme::toast())
        .into(),
    )
}

/// Renders the independent floating window from shared state.
pub fn floating(app: &App) -> Element<'_, Message> {
    float::view(app.usage(), app.float_mode())
}
