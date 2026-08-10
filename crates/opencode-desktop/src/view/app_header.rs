use crate::message::{HeaderAction, Message};
use crate::theme;
use crate::view::components::icon_button;
use crate::view::components::icons;
use crate::view::components::quota_card::QuotaHealth;
use crate::view::components::status_badge::{self, Tone};
use crate::view::format;
use iced::widget::{container, row, space, text};
use iced::{Element, Fill, Length};
use opencode_core::UsageReport;

/// Height of the control bar in logical pixels; the overflow menu's vertical
/// offset reuses it.
pub const HEIGHT: f32 = 48.0;

pub fn view(
    report: Option<&UsageReport>,
    now_ms: i64,
    loading: bool,
    tray_error: Option<&str>,
    focused_action: Option<HeaderAction>,
    menu_open: bool,
) -> Element<'static, Message> {
    let mut status: Vec<Element<'static, Message>> = vec![load_badge(report)];
    if tray_error.is_some() {
        status.push(status_badge::view("托盘不可用", Tone::Danger));
    }

    let refresh = if loading {
        icon_button::loading(
            now_ms,
            "正在刷新",
            focused_action == Some(HeaderAction::Refresh),
        )
    } else {
        icon_button::view(
            icons::REFRESH,
            "刷新",
            Message::HeaderPressed(HeaderAction::Refresh),
            focused_action == Some(HeaderAction::Refresh),
        )
    };
    let settings = icon_button::view(
        icons::SETTINGS,
        "设置",
        Message::HeaderPressed(HeaderAction::Settings),
        focused_action == Some(HeaderAction::Settings),
    );
    let float = icon_button::view(
        icons::FLOAT,
        "悬浮窗",
        Message::HeaderPressed(HeaderAction::Float),
        focused_action == Some(HeaderAction::Float),
    );
    let more = icon_button::view(
        icons::MORE,
        "更多",
        Message::HeaderPressed(HeaderAction::More),
        focused_action == Some(HeaderAction::More) || menu_open,
    );

    let mut actions: Vec<Element<'static, Message>> = Vec::new();
    if let Some(report) = report {
        actions.push(updated_text(report, now_ms, loading));
    }
    actions.push(refresh);
    actions.push(settings);
    actions.push(float);

    container(
        row![
            row(status).spacing(theme::spacing::SM),
            space::horizontal(),
            row![row(actions).spacing(4.0), more].spacing(theme::spacing::SM),
        ]
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(HEIGHT))
    .padding([6.0, theme::spacing::PAGE_PADDING])
    .style(move |_| theme::header_surface())
    .into()
}

/// The window with the highest load, if any.
fn highest(report: &UsageReport) -> Option<&opencode_core::WindowReport> {
    report.windows.iter().max_by(|a, b| {
        a.percent
            .partial_cmp(&b.percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Highest-load badge, or a neutral placeholder when there is no report yet.
fn load_badge(report: Option<&UsageReport>) -> Element<'static, Message> {
    report.and_then(highest).map_or_else(
        || status_badge::view("最高 —", Tone::Neutral),
        |window| {
            let health = QuotaHealth::from_percent(window.percent);
            status_badge::view(
                format!("最高 {}", format::percent(window.percent)),
                health.badge_tone(),
            )
        },
    )
}

/// Relative "time since last update" label; blue and explicit while loading.
fn updated_text(report: &UsageReport, now_ms: i64, loading: bool) -> Element<'static, Message> {
    if loading {
        text("正在刷新…")
            .size(theme::typography::LABEL)
            .color(theme::palette::PRIMARY)
            .into()
    } else {
        text(format!(
            "{}更新",
            format::relative(report.fetched_at, now_ms)
        ))
        .size(theme::typography::LABEL)
        .color(theme::palette::TEXT_MUTED)
        .into()
    }
}
