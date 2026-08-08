use crate::message::Message;
use crate::theme;
use crate::view::components::card;
use crate::view::components::metric;
use crate::view::components::progress_bar;
use crate::view::components::quota_card::QuotaHealth;
use crate::view::components::section_header;
use crate::view::components::status_badge::{self, Tone};
use crate::view::format;
use iced::widget::{column, container, row, text, Column, Row};
use iced::{Color, Element, Fill, FillPortion, Font, Length};
use opencode_core::UsageReport;

/// Badge text for the report data source.
fn provider_badge(report: &UsageReport) -> String {
    if report.plan_type.trim().is_empty() {
        "OpenCode Go".to_owned()
    } else {
        format!("OpenCode Go · {}", report.plan_type.to_uppercase())
    }
}

pub fn view(
    report: UsageReport,
    now_ms: i64,
    loading: bool,
    window_width: f32,
) -> Element<'static, Message> {
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
    let header = section_header::view("用量概览", trailing.into());

    let body: Element<'static, Message> = if window_width >= 900.0 {
        row![
            main_card(&report, loading).width(FillPortion(11)),
            side_cards(&report, now_ms, true).width(FillPortion(9)),
        ]
        .spacing(theme::spacing::CARD_GAP)
        .into()
    } else {
        column![
            main_card(&report, loading),
            side_cards(&report, now_ms, false),
        ]
        .spacing(theme::spacing::CARD_GAP)
        .into()
    };

    column![header, body].spacing(theme::spacing::BASE).into()
}

fn main_card(report: &UsageReport, loading: bool) -> iced::widget::Container<'static, Message> {
    let highest = report.windows.iter().max_by(|a, b| {
        a.percent
            .partial_cmp(&b.percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let Some(window) = highest else {
        return card::fixed(
            text("暂无配额数据")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_MUTED),
            268.0,
        );
    };

    let health = QuotaHealth::from_percent(window.percent);
    let accent = health.progress_color();
    let title = row![
        text("最高负载")
            .size(theme::typography::CARD_TITLE)
            .color(theme::palette::TEXT_PRIMARY)
            .width(Fill),
        status_badge::view(window.label.clone(), Tone::Neutral),
    ]
    .align_y(iced::Alignment::Center);
    let hero = row![
        text(format::percent(window.percent))
            .font(Font::MONOSPACE)
            .size(theme::typography::HERO_VALUE)
            .color(accent),
        text(if loading { "刷新中…" } else { "" })
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_MUTED),
    ]
    .spacing(theme::spacing::MD)
    .align_y(iced::Alignment::End);
    let metrics = row![
        metric::value("已用", format::number(window.used)),
        metric::value("总额", format::number(window.quota)),
        metric::value("剩余", format::number(window.remaining)),
    ]
    .spacing(theme::spacing::BASE);
    let reset = row![
        text("下次重置")
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_MUTED)
            .width(Fill),
        text(format::countdown(window.reset_in_secs))
            .size(theme::typography::BODY)
            .color(theme::palette::TEXT_PRIMARY),
    ];

    card::fixed(
        column![
            title,
            hero,
            progress_bar::view(window.percent, accent),
            metrics,
            reset
        ]
        .spacing(theme::spacing::MD),
        268.0,
    )
}

fn side_cards(
    report: &UsageReport,
    now_ms: i64,
    vertical: bool,
) -> iced::widget::Container<'static, Message> {
    let content: Element<'static, Message> = if vertical {
        Column::with_children(vec![reset_card(report, now_ms), health_card(report)])
            .spacing(theme::spacing::CARD_GAP)
            .into()
    } else {
        Row::with_children(vec![reset_card(report, now_ms), health_card(report)])
            .spacing(theme::spacing::CARD_GAP)
            .into()
    };

    container(content).width(Fill)
}

fn reset_card(report: &UsageReport, now_ms: i64) -> Element<'static, Message> {
    let next = report
        .windows
        .iter()
        .map(|window| window.reset_time.saturating_sub(now_ms) / 1_000)
        .filter(|seconds| *seconds > 0)
        .min();
    let value = next.map_or_else(|| "即将重置".to_owned(), format::countdown_short);
    let context = next.map_or("倒计时", |seconds| {
        if seconds < 86_400 {
            "今天"
        } else {
            "倒计时"
        }
    });

    card::fixed(
        column![
            text("最近重置")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_SECONDARY),
            row![
                text(value)
                    .font(Font::MONOSPACE)
                    .size(30)
                    .color(theme::palette::TEXT_PRIMARY),
                status_badge::view(context, Tone::Neutral),
            ]
            .spacing(theme::spacing::SM)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(theme::spacing::SM),
        126.0,
    )
    .into()
}

fn health_card(report: &UsageReport) -> Element<'static, Message> {
    let (healthy, warning, critical) = report.windows.iter().fold(
        (0u16, 0u16, 0u16),
        |(healthy, warning, critical), window| match QuotaHealth::from_percent(window.percent) {
            QuotaHealth::Healthy => (healthy + 1, warning, critical),
            QuotaHealth::Warning => (healthy, warning + 1, critical),
            QuotaHealth::Critical => (healthy, warning, critical + 1),
        },
    );

    card::fixed(
        column![
            text("窗口健康")
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_SECONDARY),
            row![
                count("正常", healthy, theme::palette::SUCCESS),
                count("警告", warning, theme::palette::WARNING),
                count("危险", critical, theme::palette::DANGER),
            ]
            .spacing(theme::spacing::MD),
            distribution(healthy, warning, critical),
            text(format!("共 {} 个窗口", report.windows.len()))
                .size(theme::typography::CAPTION)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(theme::spacing::SM),
        126.0,
    )
    .into()
}

fn count(label: &'static str, value: u16, color: Color) -> Element<'static, Message> {
    row![
        text("●").size(9).color(color),
        text(format!("{label} {value}"))
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_SECONDARY),
    ]
    .spacing(theme::spacing::XS)
    .align_y(iced::Alignment::Center)
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
