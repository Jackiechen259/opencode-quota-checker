use crate::config::FloatMode;
use crate::message::Message;
use crate::state::UsageState;
use crate::theme;
use crate::view::components::quota_card::QuotaHealth;
use crate::view::format;
use iced::widget::{button, column, container, mouse_area, progress_bar, row, text, Column};
use iced::{Element, Fill};
use volc_core::WindowReport;

/// Renders the floating window from the shared usage state.
pub fn view(state: &UsageState, mode: FloatMode) -> Element<'_, Message> {
    let drag_bar = mouse_area(
        container(
            text("VOLC Status · 拖动区域")
                .size(11)
                .color(theme::palette::TEXT_MUTED),
        )
        .width(Fill)
        .padding([6, 10]),
    )
    .on_press(Message::DragFloat);
    let body: Element<'_, Message> = match mode {
        FloatMode::Full => full(state),
        FloatMode::Compact => compact(state),
        FloatMode::Docked => docked(state),
    };
    container(column![drag_bar, body].spacing(6))
        .width(Fill)
        .height(Fill)
        .padding(8)
        .style(move |_| theme::float_card())
        .into()
}

fn full(state: &UsageState) -> Element<'_, Message> {
    let controls = row![
        button("精简")
            .on_press(Message::FloatModeChanged(FloatMode::Compact))
            .style(theme::soft_button)
            .padding([6, 12]),
        button("吸附")
            .on_press(Message::FloatModeChanged(FloatMode::Docked))
            .style(theme::soft_button)
            .padding([6, 12]),
        button("刷新")
            .on_press(Message::Refresh)
            .style(button::primary)
            .padding([6, 12]),
        button("关闭")
            .on_press(Message::CloseFloat)
            .style(theme::soft_button)
            .padding([6, 12]),
    ]
    .spacing(6);
    let mut content = column![controls].spacing(10);
    if let Some(report) = &state.report {
        content = content.push(
            text(format!("套餐：{}", report.plan_type))
                .size(15)
                .color(theme::palette::TEXT_PRIMARY),
        );
        for window in &report.windows {
            content = content.push(window_line(window));
        }
        content = content.push(
            text(format!(
                "更新：{}",
                format::relative(report.fetched_at, state.now_ms)
            ))
            .size(11)
            .color(theme::palette::TEXT_MUTED),
        );
    } else {
        content = content.push(text("等待用量数据…").color(theme::palette::TEXT_MUTED));
    }
    content.into()
}

fn compact(state: &UsageState) -> Element<'_, Message> {
    let Some(window) = highest(state) else {
        return column![
            row![
                button("展开")
                    .on_press(Message::FloatModeChanged(FloatMode::Full))
                    .style(theme::soft_button)
                    .padding([6, 12]),
                button("关闭")
                    .on_press(Message::CloseFloat)
                    .style(theme::soft_button)
                    .padding([6, 12]),
            ]
            .spacing(6),
            text("等待用量数据…").color(theme::palette::TEXT_MUTED),
        ]
        .spacing(8)
        .into();
    };
    let health = QuotaHealth::from_percent(window.percent);
    let bar_color = health.color();
    column![
        row![
            text(format!("{} · {:.1}%", window.label, window.percent))
                .size(16)
                .color(theme::palette::TEXT_PRIMARY),
            button("展开")
                .on_press(Message::FloatModeChanged(FloatMode::Full))
                .style(theme::soft_button)
                .padding([6, 12]),
            button("关闭")
                .on_press(Message::CloseFloat)
                .style(theme::soft_button)
                .padding([6, 12]),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
        progress_bar(0.0..=100.0, window.percent as f32)
            .girth(7)
            .style(move |_| theme::progress_style(bar_color)),
        text(format!("剩余 {:.1}", window.remaining))
            .size(11)
            .color(theme::palette::TEXT_MUTED),
    ]
    .spacing(8)
    .into()
}

fn docked(state: &UsageState) -> Element<'_, Message> {
    let (status, color) = highest(state).map_or_else(
        || ("● 等待数据".to_owned(), theme::palette::TEXT_MUTED),
        |window| {
            (
                format!("● {} {:.1}%", window.label, window.percent),
                QuotaHealth::from_percent(window.percent).color(),
            )
        },
    );
    row![
        text(status).width(Fill).color(color),
        button("展开")
            .on_press(Message::FloatModeChanged(FloatMode::Full))
            .style(theme::soft_button)
            .padding([4, 10]),
        button("×")
            .on_press(Message::CloseFloat)
            .style(theme::soft_button)
            .padding([4, 10]),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .into()
}

fn highest(state: &UsageState) -> Option<&WindowReport> {
    state.report.as_ref()?.windows.iter().max_by(|left, right| {
        left.percent
            .partial_cmp(&right.percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn window_line(window: &WindowReport) -> Element<'static, Message> {
    let health = QuotaHealth::from_percent(window.percent);
    let bar_color = health.color();
    Column::new()
        .push(
            row![
                text(window.label.clone())
                    .width(Fill)
                    .color(theme::palette::TEXT_PRIMARY),
                text(format!("{:.1}%", window.percent)).color(health.color()),
            ]
            .spacing(8),
        )
        .push(
            progress_bar(0.0..=100.0, window.percent as f32)
                .girth(6)
                .style(move |_| theme::progress_style(bar_color)),
        )
        .spacing(4)
        .into()
}
