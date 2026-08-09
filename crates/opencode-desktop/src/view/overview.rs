use crate::message::Message;
use crate::theme;
use crate::view::components::quota_card::QuotaHealth;
use crate::view::components::status_badge::{self, Tone};
use crate::view::components::{card, divider, dot, icons, metric, progress_bar, section_header};
use crate::view::format;
use iced::widget::{column, container, row, rule, svg, text, Row};
use iced::{Color, Element, Fill, FillPortion, Length};
use opencode_core::{UsageReport, WindowReport};

/// Badge text for the report data source.
fn provider_badge(report: &UsageReport) -> String {
    if report.plan_type.trim().is_empty() {
        "OpenCode Go".to_owned()
    } else {
        format!("OpenCode Go · {}", report.plan_type.to_uppercase())
    }
}

pub fn view(report: UsageReport, now_ms: i64) -> Element<'static, Message> {
    let trailing = row![
        status_badge::view(provider_badge(&report), Tone::Primary),
        text("·")
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_MUTED),
        text(format!(
            "{}更新",
            format::relative(report.fetched_at, now_ms)
        ))
        .size(theme::typography::LABEL)
        .color(theme::palette::TEXT_MUTED),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);
    let header = section_header::with_icon(
        icons::ACTIVITY,
        "用量概览",
        "当前额度窗口的整体负载",
        trailing.into(),
    );

    let highest = report.windows.iter().max_by(|a, b| {
        a.percent
            .partial_cmp(&b.percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let body: Element<'static, Message> = match highest {
        Some(window) => hero_card(window, &report),
        None => card::standard(
            text("暂无配额数据")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_MUTED),
        )
        .into(),
    };

    column![header, body].spacing(theme::spacing::BASE).into()
}

/// A single content-adaptive hero card for the most-loaded window.
fn hero_card(window: &WindowReport, report: &UsageReport) -> Element<'static, Message> {
    let health = QuotaHealth::from_percent(window.percent);
    let accent = health.progress_color();
    let (healthy, warning, critical) =
        report
            .windows
            .iter()
            .fold((0u16, 0u16, 0u16), |(healthy, warning, critical), w| {
                match QuotaHealth::from_percent(w.percent) {
                    QuotaHealth::Healthy => (healthy + 1, warning, critical),
                    QuotaHealth::Warning => (healthy, warning + 1, critical),
                    QuotaHealth::Critical => (healthy, warning, critical + 1),
                }
            });

    let heading = row![
        dot::view(health.status_color(), 9.0),
        text("最高负载")
            .size(theme::typography::CARD_TITLE)
            .color(theme::palette::TEXT_PRIMARY),
        status_badge::view(window.label.clone(), Tone::Neutral),
        row![].width(Fill),
    ]
    .spacing(theme::spacing::SM)
    .align_y(iced::Alignment::Center);

    let clock = svg(icons::handle(icons::CLOCK))
        .style(move |_theme, _status| svg::Style {
            color: Some(theme::palette::TEXT_MUTED),
        })
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0));
    let hero = row![
        text(format::percent(window.percent))
            .font(theme::typography::ui_semibold())
            .size(theme::typography::HERO_VALUE)
            .color(accent),
        row![].width(Fill),
        row![
            clock,
            text(format::countdown(window.reset_in_secs))
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_PRIMARY),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    ]
    .align_y(iced::Alignment::End);

    let stats = row![
        metric::value("已用", format::number(window.used)),
        divider::vertical(40.0),
        metric::value("总额", format::number(window.quota)),
        divider::vertical(40.0),
        metric::value("剩余", format::number(window.remaining)),
        divider::vertical(40.0),
        column![
            metric::value(
                "窗口健康",
                format!("{} / {}", healthy, report.windows.len())
            ),
            distribution(healthy, warning, critical),
        ]
        .spacing(theme::spacing::SM)
        .width(Fill),
    ]
    .align_y(iced::Alignment::Center);

    card::standard(
        column![
            heading,
            hero,
            progress_bar::view(window.percent, accent, 10.0),
            rule::horizontal(1).style(rule_style),
            stats,
        ]
        .spacing(theme::spacing::MD),
    )
    .into()
}

fn distribution(healthy: u16, warning: u16, critical: u16) -> Element<'static, Message> {
    let total = healthy + warning + critical;
    if total == 0 {
        return segment(theme::palette::TRACK, 1);
    }

    let mut segments = Row::new().spacing(2).width(Fill);
    for (count, color) in [
        (healthy, theme::palette::SUCCESS),
        (warning, theme::palette::WARNING),
        (critical, theme::palette::DANGER),
    ] {
        if count > 0 {
            segments = segments.push(segment(color, count));
        }
    }
    segments.into()
}

fn segment(color: Color, portion: u16) -> Element<'static, Message> {
    container(row![])
        .width(FillPortion(portion))
        .height(Length::Fixed(6.0))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::border::rounded(theme::radius::PILL),
            ..container::Style::default()
        })
        .into()
}

fn rule_style(_theme: &iced::Theme) -> rule::Style {
    rule::Style {
        color: theme::palette::DIVIDER,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: false,
    }
}
