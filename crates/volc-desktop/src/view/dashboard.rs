use crate::message::Message;
use crate::state::UsageState;
use crate::view::components::quota_card::{self, QuotaHealth};
use crate::view::format;
use iced::widget::{button, column, container, responsive, row, text, Column, Row};
use iced::{Element, Fill};
use volc_core::UsageReport;

/// Renders summaries, quota cards, request states, and the raw response.
pub fn view(state: &UsageState) -> Element<'_, Message> {
    let refresh = if state.loading {
        button("刷新中…")
    } else {
        button("刷新").on_press(Message::Refresh)
    };
    let raw = if state.raw_loading {
        button("读取原始响应中…")
    } else {
        button("查看原始 JSON").on_press(Message::LoadRaw)
    };
    let actions = row![
        refresh,
        raw,
        button("删除凭证").on_press(Message::ClearCredentials)
    ]
    .spacing(10);
    let mut content = column![actions].spacing(16);

    match &state.report {
        Some(report) => {
            content = content
                .push(summary(report, state.now_ms))
                .push(cards(report, state.now_ms))
                .push(text(format!(
                    "最后更新：{}{}",
                    format::timestamp(report.fetched_at),
                    if state.loading {
                        " · 正在刷新"
                    } else {
                        ""
                    }
                )));
        }
        None if state.loading => {
            content = content.push(text("正在安全地加载用量数据…").size(20));
        }
        None => {
            content = content.push(text("暂无可显示的配额窗口，请尝试刷新。").size(20));
        }
    }
    if let Some(error) = &state.error {
        content = content.push(
            container(
                column![
                    text(&error.user).size(16),
                    text(format!("技术详情：{}", error.detail)).size(12)
                ]
                .spacing(6),
            )
            .padding(12),
        );
    }
    content.into()
}

fn summary(report: &UsageReport, now_ms: i64) -> Element<'static, Message> {
    let highest = report.windows.iter().max_by(|left, right| {
        left.percent
            .partial_cmp(&right.percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let healthy = report
        .windows
        .iter()
        .filter(|window| QuotaHealth::from_percent(window.percent) == QuotaHealth::Healthy)
        .count();
    let next_reset = report
        .windows
        .iter()
        .map(|window| window.reset_time.saturating_sub(now_ms) / 1_000)
        .filter(|seconds| *seconds > 0)
        .min();

    row![
        summary_metric(
            "最高负载",
            highest.map_or_else(
                || "暂无".to_owned(),
                |window| format!("{} · {:.1}%", window.label, window.percent)
            )
        ),
        summary_metric("健康窗口", format!("{healthy} / {}", report.windows.len())),
        summary_metric(
            "最近重置",
            next_reset.map_or_else(|| "即将重置".to_owned(), format::countdown)
        ),
    ]
    .spacing(12)
    .into()
}

fn summary_metric(label: &'static str, value: String) -> Element<'static, Message> {
    container(column![text(label).size(13), text(value).size(19)].spacing(5))
        .width(Fill)
        .padding(14)
        .style(container::rounded_box)
        .into()
}

fn cards(report: &UsageReport, now_ms: i64) -> Element<'static, Message> {
    let windows = report.windows.clone();
    responsive(move |size| {
        let cards = windows
            .iter()
            .cloned()
            .map(|window| quota_card::view(window, now_ms))
            .collect::<Vec<_>>();
        if size.width < 760.0 {
            Column::with_children(cards).spacing(12).into()
        } else {
            Row::with_children(cards).spacing(12).into()
        }
    })
    .into()
}
