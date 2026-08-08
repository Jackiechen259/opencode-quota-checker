use crate::message::{HeaderAction, Message};
use crate::theme;
use crate::view::components::{icon_button, icons};
use crate::view::format;
use iced::widget::{column, container, responsive, row, space, text};
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
    responsive(move |size| {
        layout(
            report.as_ref(),
            now_ms,
            loading,
            tray_online,
            focused_action,
            details_enabled,
            size.width,
        )
    })
    .height(Length::Shrink)
    .into()
}

fn layout(
    report: Option<&UsageReport>,
    now_ms: i64,
    loading: bool,
    tray_online: bool,
    focused_action: Option<HeaderAction>,
    details_enabled: bool,
    width: f32,
) -> Element<'static, Message> {
    let brand = brand(tray_online);
    let metrics = metrics_row(report, now_ms);
    let actions = actions(loading, now_ms, focused_action, details_enabled);

    let content: Element<'static, Message> = if width >= 980.0 {
        row![brand, space::horizontal(), metrics, actions]
            .spacing(theme::spacing::LG)
            .align_y(iced::Alignment::Center)
            .into()
    } else {
        column![
            row![brand, space::horizontal(), actions]
                .spacing(theme::spacing::MD)
                .align_y(iced::Alignment::Center),
            metrics,
        ]
        .spacing(theme::spacing::SM)
        .into()
    };

    container(content)
        .width(Fill)
        .height(Length::Fixed(if width >= 980.0 { 96.0 } else { 132.0 }))
        .padding([14.0, theme::spacing::PAGE_PADDING])
        .style(move |_| theme::header_surface())
        .into()
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
    let logo = container(
        text("OC")
            .size(theme::typography::CARD_TITLE)
            .color(theme::palette::SURFACE),
    )
    .width(Length::Fixed(44.0))
    .height(Length::Fixed(44.0))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_| theme::logo());

    let meta = row![
        text("OpenCode Go · 配额监控")
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_SECONDARY),
        text("●").size(9).color(tray_color),
        text(tray_text)
            .size(theme::typography::CAPTION)
            .color(theme::palette::TEXT_SECONDARY),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    row![
        logo,
        column![
            text("OpenCode Quota Checker")
                .size(theme::typography::PAGE_TITLE)
                .color(theme::palette::TEXT_PRIMARY),
            meta,
        ]
        .spacing(theme::spacing::XS),
    ]
    .spacing(theme::spacing::MD)
    .align_y(iced::Alignment::Center)
    .width(Length::Shrink)
    .into()
}

fn metrics_row(report: Option<&UsageReport>, now_ms: i64) -> Element<'static, Message> {
    let (highest, healthy, total, next_reset) =
        report.map_or((None, 0usize, 0usize, None), |report| {
            let highest = report.windows.iter().max_by(|a, b| {
                a.percent
                    .partial_cmp(&b.percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let healthy = report.windows.iter().filter(|w| w.percent < 70.0).count();
            let next = report
                .windows
                .iter()
                .map(|w| w.reset_time.saturating_sub(now_ms) / 1_000)
                .filter(|seconds| *seconds > 0)
                .min();
            (highest, healthy, report.windows.len(), next)
        });

    row![
        metric(
            "最高负载",
            highest.map_or_else(|| "—".to_owned(), |window| format::percent(window.percent))
        ),
        divider(),
        metric("窗口健康", format!("{healthy} / {total}")),
        divider(),
        metric(
            "下次重置",
            next_reset.map_or_else(|| "—".to_owned(), format::countdown_short)
        ),
    ]
    .spacing(theme::spacing::BASE)
    .align_y(iced::Alignment::Center)
    .into()
}

fn metric(label: &'static str, value: String) -> Element<'static, Message> {
    column![
        text(label)
            .size(theme::typography::CAPTION)
            .color(theme::palette::TEXT_MUTED),
        text(value).size(16).color(theme::palette::TEXT_PRIMARY),
    ]
    .spacing(2)
    .into()
}

fn divider() -> Element<'static, Message> {
    container(row![])
        .width(Length::Fixed(1.0))
        .height(Length::Fixed(28.0))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(theme::palette::DIVIDER)),
            ..container::Style::default()
        })
        .into()
}

fn actions(
    loading: bool,
    now_ms: i64,
    focused_action: Option<HeaderAction>,
    details_enabled: bool,
) -> Element<'static, Message> {
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

    row(buttons)
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
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
