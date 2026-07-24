//! The top application header: brand, compact metrics, and icon buttons.

use crate::message::Message;
use crate::theme;
use crate::view::components::icons;
use crate::view::format;
use iced::widget::tooltip::Position;
use iced::widget::{button, column, container, row, svg, text, tooltip};
use iced::{Element, Fill};
use volc_core::UsageReport;

/// Renders the header row for the main window.
pub fn view(
    report: Option<&UsageReport>,
    now_ms: i64,
    loading: bool,
    tray_error: Option<&str>,
) -> Element<'static, Message> {
    let tray_status = tray_error.map_or_else(
        || "托盘在线".to_owned(),
        |error| format!("托盘不可用：{error}"),
    );
    let brand = column![
        text("VOLC Status")
            .size(24)
            .color(theme::palette::TEXT_PRIMARY),
        text("Agent Plan · AFP 配额")
            .size(13)
            .color(theme::palette::TEXT_MUTED),
        text(tray_status).size(11).color(theme::palette::TEXT_MUTED),
    ]
    .spacing(2)
    .width(Fill);

    let logo = container(text("V").size(24).color(theme::palette::SURFACE))
        .width(iced::Length::Fixed(48.0))
        .height(iced::Length::Fixed(48.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_| theme::logo());

    let metrics = metrics_row(report, now_ms);

    let actions = row![
        icon_button(icons::REFRESH, "刷新", Some(Message::Refresh), loading),
        icon_button(icons::CODE, "原始 JSON", Some(Message::LoadRaw), false),
        icon_button(icons::SETTINGS, "设置", Some(Message::OpenSettings), false),
        icon_button(icons::FLOAT, "悬浮窗", Some(Message::ToggleFloat), false),
        icon_button(icons::HIDE, "隐藏", Some(Message::HideMain), false),
        icon_button(icons::EXIT, "退出", Some(Message::Exit), false),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    container(
        row![row![logo, brand].spacing(14).width(Fill), metrics, actions]
            .spacing(20)
            .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .padding([16, 32])
    .style(move |_| theme::header_surface())
    .into()
}

/// The three compact header metrics with vertical dividers between them.
fn metrics_row(report: Option<&UsageReport>, now_ms: i64) -> Element<'static, Message> {
    let (highest, healthy, total, next_reset) =
        report.map_or((None, 0u32, 0usize, None), |report| {
            let highest = report.windows.iter().max_by(|a, b| {
                a.percent
                    .partial_cmp(&b.percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let healthy = report.windows.iter().filter(|w| w.percent < 70.0).count() as u32;
            let total = report.windows.len();
            let next = report
                .windows
                .iter()
                .map(|w| w.reset_time.saturating_sub(now_ms) / 1_000)
                .filter(|s| *s > 0)
                .min();
            (highest, healthy, total, next)
        });

    let highest_val = highest.map_or_else(|| "—".to_owned(), |w| format::percent(w.percent));
    let next_val = next_reset.map_or_else(|| "—".to_owned(), format::countdown_short);

    row![
        metric("最高负载", highest_val),
        divider(),
        metric("窗口健康", format!("{healthy}/{total}")),
        divider(),
        metric("下次重置", next_val),
    ]
    .spacing(16)
    .align_y(iced::Alignment::Center)
    .into()
}

fn metric(label: &'static str, value: String) -> Element<'static, Message> {
    column![
        text(label).size(11).color(theme::palette::TEXT_MUTED),
        text(value).size(16).color(theme::palette::TEXT_PRIMARY),
    ]
    .spacing(2)
    .into()
}

fn divider() -> Element<'static, Message> {
    container(row![])
        .width(iced::Length::Fixed(1.0))
        .height(iced::Length::Fixed(28.0))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(theme::palette::DIVIDER)),
            ..container::Style::default()
        })
        .into()
}

/// A square ghost icon button with a tooltip. Disabled (no `message`) while
/// `loading` is true.
fn icon_button(
    icon: &'static [u8],
    label: &'static str,
    message: Option<Message>,
    loading: bool,
) -> Element<'static, Message> {
    let color = if loading {
        theme::palette::TEXT_MUTED
    } else {
        theme::palette::TEXT_SECONDARY
    };
    let icon_handle = icons::handle(icon);
    let mut btn = button(
        svg(icon_handle)
            .style(move |_theme, _status| svg::Style { color: Some(color) })
            .width(iced::Length::Fixed(20.0))
            .height(iced::Length::Fixed(20.0)),
    )
    .padding(10)
    .style(theme::icon_button);

    let content: Element<'static, Message> = if loading {
        btn.into()
    } else if let Some(msg) = message {
        btn = btn.on_press(msg);
        btn.into()
    } else {
        btn.into()
    };

    tooltip(
        content,
        container(text(label).size(12).color(theme::palette::SURFACE))
            .padding([6, 10])
            .style(move |_| theme::tooltip_surface()),
        Position::Bottom,
    )
    .into()
}
