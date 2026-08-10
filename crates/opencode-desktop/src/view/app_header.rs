use crate::message::{HeaderAction, Message};
use crate::theme;
use crate::view::components::dot;
use crate::view::components::quota_card::QuotaHealth;
use crate::view::components::status_badge::{self, Tone};
use crate::view::components::{icon_button, icons};
use crate::view::format;
use iced::widget::{container, row, space, text, Column};
use iced::{Element, Fill, Length};
use opencode_core::UsageReport;

pub fn view(
    report: Option<&UsageReport>,
    now_ms: i64,
    loading: bool,
    tray_error: Option<&str>,
    focused_action: Option<HeaderAction>,
    details_enabled: bool,
) -> Element<'static, Message> {
    let report = report.cloned();
    let tray_online = tray_error.is_none();
    let load = report.as_ref().and_then(highest).map_or_else(
        || status_badge::view("最高 —", Tone::Neutral),
        |window| {
            let health = QuotaHealth::from_percent(window.percent);
            status_badge::view(
                format!("最高 {}", format::percent(window.percent)),
                health.badge_tone(),
            )
        },
    );

    container(
        row![
            brand(tray_online),
            space::horizontal(),
            load,
            row(actions(loading, now_ms, focused_action, details_enabled))
                .spacing(6)
                .align_y(iced::Alignment::Center),
        ]
        .spacing(theme::spacing::LG)
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(72.0))
    .padding([12.0, theme::spacing::PAGE_PADDING])
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

fn brand(tray_online: bool) -> Element<'static, Message> {
    let tray_color = if tray_online {
        theme::palette::SUCCESS
    } else {
        theme::palette::DANGER
    };
    let tray_text = if tray_online {
        "托盘在线"
    } else {
        "托盘不可用"
    };
    let logo = crate::view::components::app_icon::view(48.0);

    let meta = row![
        text("OpenCode Go · 配额监控")
            .size(theme::typography::CAPTION)
            .color(theme::palette::TEXT_SECONDARY),
        dot::view(tray_color, 9.0),
        text(tray_text)
            .size(theme::typography::CAPTION)
            .color(theme::palette::TEXT_SECONDARY),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let title = text("OpenCode Quota Checker")
        .size(17)
        .font(theme::typography::ui_medium())
        .color(theme::palette::TEXT_PRIMARY);
    let name = Column::new()
        .push(title)
        .push(meta)
        .spacing(theme::spacing::XS);

    row![logo, name]
        .spacing(theme::spacing::MD)
        .align_y(iced::Alignment::Center)
        .width(Length::Shrink)
        .into()
}

fn actions(
    loading: bool,
    now_ms: i64,
    focused_action: Option<HeaderAction>,
    details_enabled: bool,
) -> Vec<Element<'static, Message>> {
    let refresh = if loading {
        icon_button::loading(
            now_ms,
            "正在刷新",
            focused_action == Some(HeaderAction::Refresh),
        )
    } else {
        action_button(
            icons::REFRESH,
            "刷新",
            HeaderAction::Refresh,
            focused_action,
        )
    };

    let mut buttons = vec![refresh];
    if details_enabled {
        buttons.push(action_button(
            icons::CODE,
            "开发者详情",
            HeaderAction::Details,
            focused_action,
        ));
    }
    buttons.extend([
        action_button(
            icons::SETTINGS,
            "设置",
            HeaderAction::Settings,
            focused_action,
        ),
        action_button(icons::FLOAT, "悬浮窗", HeaderAction::Float, focused_action),
        action_button(
            icons::HIDE,
            "隐藏到托盘",
            HeaderAction::Hide,
            focused_action,
        ),
        action_button(icons::EXIT, "退出", HeaderAction::Exit, focused_action),
    ]);
    buttons
}

fn action_button(
    icon: &'static [u8],
    label: &'static str,
    action: HeaderAction,
    focused_action: Option<HeaderAction>,
) -> Element<'static, Message> {
    icon_button::view(
        icon,
        label,
        Message::HeaderPressed(action),
        focused_action == Some(action),
    )
}
