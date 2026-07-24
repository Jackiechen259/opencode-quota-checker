use crate::message::Message;
use crate::state::UsageState;
use crate::theme;
use crate::view::components::quota_card::{self, CardLayout, QuotaHealth};
use crate::view::format;
use iced::alignment::Horizontal;
use iced::widget::{column, container, progress_bar, responsive, row, text, Column, Row};
use iced::{Color, Element, Fill};
use volc_core::UsageReport;

/// Renders the overview, detail cards, and transient states.
pub fn view(state: &UsageState) -> Element<'_, Message> {
    let mut content = column![].spacing(24);

    if let Some(report) = &state.report {
        let report = report.clone();
        let now_ms = state.now_ms;
        let loading = state.loading;
        let report_overview = report.clone();
        let report_details = report.clone();
        content = content.push(responsive(move |size| {
            overview(report_overview.clone(), now_ms, loading, size.width)
        }));
        content = content.push(responsive(move |size| {
            details(report_details.clone(), now_ms, size.width)
        }));
        content = content.push(
            text(format!(
                "最后更新：{}（{}）{}",
                format::timestamp(report.fetched_at),
                format::relative(report.fetched_at, state.now_ms),
                if state.loading {
                    " · 正在刷新"
                } else {
                    ""
                }
            ))
            .size(12)
            .color(theme::palette::TEXT_MUTED),
        );
    } else if state.loading {
        content = content.push(skeleton());
    } else {
        content = content.push(empty_state("暂无可显示的配额窗口，请尝试刷新。"));
    }

    if let Some(error) = &state.error {
        content = content.push(error_notice(error.user.clone(), error.detail.clone()));
    }
    content.into()
}

/// Title row: `用量概览 [PLAN]` on the left, last-updated on the right.
fn title_row(report: &UsageReport, now_ms: i64, width: f32) -> Element<'static, Message> {
    let left = row![
        text("用量概览")
            .size(20)
            .color(theme::palette::TEXT_PRIMARY)
            .width(Fill),
        plan_badge(&report.plan_type),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let right_text = if width < 520.0 {
        format::relative(report.fetched_at, now_ms)
    } else {
        format!("最后更新 {}", format::relative(report.fetched_at, now_ms))
    };
    let right = text(right_text).size(12).color(theme::palette::TEXT_MUTED);

    row![left, right]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
}

/// Pill-shaped plan badge.
fn plan_badge(plan: &str) -> Element<'static, Message> {
    container(
        text(plan.to_uppercase())
            .size(11)
            .color(theme::palette::PRIMARY),
    )
    .padding([4, 10])
    .style(move |_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba8(
            59, 130, 246, 0.10,
        ))),
        border: iced::Border {
            color: Color::from_rgba8(59, 130, 246, 0.25),
            width: 1.0,
            radius: theme::radius::PILL.into(),
        },
        ..container::Style::default()
    })
    .into()
}

/// Overview region: main highest-load card + side cards (最近重置, 窗口健康).
fn overview(
    report: UsageReport,
    now_ms: i64,
    loading: bool,
    width: f32,
) -> Element<'static, Message> {
    let title = title_row(&report, now_ms, width);
    let main = main_card(&report, loading);
    let side = column![reset_card(&report, now_ms), health_card(&report)].spacing(16);

    let body: Element<'static, Message> = if width >= 900.0 {
        row![main, side].spacing(16).into()
    } else {
        column![main, side].spacing(16).into()
    };
    column![title, body].spacing(16).into()
}

/// Large card for the highest-load window: big percent, progress, metrics.
fn main_card(report: &UsageReport, loading: bool) -> Element<'static, Message> {
    let highest = report.windows.iter().max_by(|a, b| {
        a.percent
            .partial_cmp(&b.percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let Some(window) = highest else {
        return container(text("暂无配额数据").color(theme::palette::TEXT_MUTED))
            .width(Fill)
            .padding(24)
            .style(move |_| theme::large_card())
            .into();
    };
    let health = QuotaHealth::from_percent(window.percent);
    let bar_color = health.color();

    let header = row![
        text("最高负载").size(13).color(theme::palette::TEXT_MUTED),
        text(format!("· {}", window.label))
            .size(13)
            .color(theme::palette::TEXT_SECONDARY),
        text(if loading { "  刷新中…" } else { "" })
            .size(12)
            .color(theme::palette::TEXT_MUTED),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let big_percent = text(format::percent(window.percent))
        .size(56)
        .color(health.color());

    let bar = progress_bar(0.0..=100.0, window.percent as f32)
        .girth(10)
        .style(move |_| theme::progress_style(bar_color));

    let metrics = row![
        big_metric("已用", format::number(window.used)),
        big_metric("总额", format::number(window.quota)),
        big_metric("剩余", format::number(window.remaining)),
    ]
    .spacing(20);

    container(
        column![
            header,
            big_percent,
            bar,
            metrics,
            text(format!(
                "下次重置：{}",
                format::countdown(window.reset_in_secs)
            ))
            .size(12)
            .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(14),
    )
    .width(Fill)
    .padding(28)
    .style(move |_| theme::large_card())
    .into()
}

/// Two-line metric: label + large value.
fn big_metric(label: &'static str, value: String) -> Element<'static, Message> {
    column![
        text(label).size(12).color(theme::palette::TEXT_MUTED),
        text(value).size(20).color(theme::palette::TEXT_PRIMARY),
    ]
    .spacing(4)
    .into()
}

/// Side card: nearest reset countdown.
fn reset_card(report: &UsageReport, now_ms: i64) -> Element<'static, Message> {
    let next = report
        .windows
        .iter()
        .map(|w| w.reset_time.saturating_sub(now_ms) / 1_000)
        .filter(|s| *s > 0)
        .min();
    let value = next.map_or_else(|| "即将重置".to_owned(), format::countdown_short);
    container(
        column![
            text("最近重置").size(13).color(theme::palette::TEXT_MUTED),
            text(value).size(28).color(theme::palette::TEXT_PRIMARY),
        ]
        .spacing(8),
    )
    .width(Fill)
    .padding(20)
    .style(move |_| theme::card())
    .into()
}

/// Side card: window health counts (正常 · 告警 · 危险).
fn health_card(report: &UsageReport) -> Element<'static, Message> {
    let (healthy, warning, critical) =
        report
            .windows
            .iter()
            .fold(
                (0u32, 0u32, 0u32),
                |(h, w, c), window| match QuotaHealth::from_percent(window.percent) {
                    QuotaHealth::Healthy => (h + 1, w, c),
                    QuotaHealth::Warning => (h, w + 1, c),
                    QuotaHealth::Critical => (h, w, c + 1),
                },
            );
    let counts = row![
        count_chip("正常", healthy, theme::palette::SUCCESS),
        count_chip("告警", warning, theme::palette::WARNING),
        count_chip("危险", critical, theme::palette::DANGER),
    ]
    .spacing(12);

    container(
        column![
            text("窗口健康").size(13).color(theme::palette::TEXT_MUTED),
            counts,
            text(format!("共 {} 个窗口", report.windows.len()))
                .size(12)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(10),
    )
    .width(Fill)
    .padding(20)
    .style(move |_| theme::card())
    .into()
}

fn count_chip(label: &str, count: u32, color: Color) -> Element<'static, Message> {
    row![
        text("●").size(10).color(color),
        text(format!("{label} {count}"))
            .size(13)
            .color(theme::palette::TEXT_SECONDARY),
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center)
    .into()
}

/// Detail region: header + responsive quota cards.
fn details(report: UsageReport, now_ms: i64, width: f32) -> Element<'static, Message> {
    let header = row![
        text("详细指标")
            .size(18)
            .color(theme::palette::TEXT_PRIMARY)
            .width(Fill),
        text(format!("共 {} 个窗口", report.windows.len()))
            .size(12)
            .color(theme::palette::TEXT_MUTED),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let layout = if width >= 760.0 {
        CardLayout::Wide
    } else {
        CardLayout::Narrow
    };
    let cards = report.windows.clone();
    let cards_view: Element<'_, Message> = responsive(move |size| {
        let children = cards
            .iter()
            .cloned()
            .map(|w| quota_card::view(w, now_ms, layout))
            .collect::<Vec<_>>();
        if size.width >= 760.0 {
            Row::with_children(children).spacing(16).into()
        } else {
            Column::with_children(children).spacing(16).into()
        }
    })
    .into();

    column![header, cards_view].spacing(16).into()
}

fn skeleton() -> Element<'static, Message> {
    container(
        column![
            text("正在安全地加载用量数据…")
                .size(16)
                .color(theme::palette::TEXT_MUTED),
            text("首次加载期间保持页面结构稳定。")
                .size(12)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(6)
        .align_x(Horizontal::Center),
    )
    .width(Fill)
    .padding(48)
    .align_x(Horizontal::Center)
    .style(move |_| theme::card())
    .into()
}

fn empty_state(message: &str) -> Element<'static, Message> {
    container(
        text(message.to_owned())
            .size(16)
            .color(theme::palette::TEXT_MUTED),
    )
    .width(Fill)
    .padding(48)
    .align_x(Horizontal::Center)
    .style(move |_| theme::card())
    .into()
}

/// Non-blocking error notice keeping any previous data visible.
fn error_notice(user: String, detail: String) -> Element<'static, Message> {
    container(
        column![
            text(format!("暂时无法更新配额数据：{user}"))
                .size(14)
                .color(theme::palette::TEXT_PRIMARY),
            text(format!("技术详情：{detail}"))
                .size(12)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(4),
    )
    .width(Fill)
    .padding(14)
    .style(move |_| theme::warning_box())
    .into()
}
