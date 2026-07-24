use crate::config::FloatMode;
use crate::message::Message;
use crate::state::UsageState;
use crate::view::format;
use iced::widget::{button, column, container, mouse_area, progress_bar, row, text, Column};
use iced::{Element, Fill};
use volc_core::WindowReport;

/// Renders the floating window from the shared usage state.
pub fn view(state: &UsageState, mode: FloatMode) -> Element<'_, Message> {
    let drag_bar = mouse_area(
        container(text("VOLC Status · 拖动区域").size(12))
            .width(Fill)
            .padding([5, 8]),
    )
    .on_press(Message::DragFloat);
    let body: Element<'_, Message> = match mode {
        FloatMode::Full => full(state),
        FloatMode::Compact => compact(state),
        FloatMode::Docked => docked(state),
    };
    container(column![drag_bar, body].spacing(5))
        .width(Fill)
        .height(Fill)
        .padding(6)
        .into()
}

fn full(state: &UsageState) -> Element<'_, Message> {
    let controls = row![
        button("精简").on_press(Message::FloatModeChanged(FloatMode::Compact)),
        button("吸附").on_press(Message::FloatModeChanged(FloatMode::Docked)),
        button("刷新").on_press(Message::Refresh),
        button("关闭").on_press(Message::CloseFloat)
    ]
    .spacing(6);
    let mut content = column![controls].spacing(8);
    if let Some(report) = &state.report {
        content = content.push(text(format!("套餐：{}", report.plan_type)).size(18));
        for window in &report.windows {
            content = content.push(window_line(window));
        }
        content = content.push(text(format!(
            "更新：{}",
            format::timestamp(report.fetched_at)
        )));
    } else {
        content = content.push(text("等待用量数据…"));
    }
    content.into()
}

fn compact(state: &UsageState) -> Element<'_, Message> {
    let Some(window) = highest(state) else {
        return column![
            row![
                button("展开").on_press(Message::FloatModeChanged(FloatMode::Full)),
                button("关闭").on_press(Message::CloseFloat)
            ]
            .spacing(6),
            text("等待用量数据…")
        ]
        .spacing(8)
        .into();
    };
    column![
        row![
            text(format!("{} · {:.1}%", window.label, window.percent)).size(20),
            button("展开").on_press(Message::FloatModeChanged(FloatMode::Full)),
            button("关闭").on_press(Message::CloseFloat)
        ]
        .spacing(6),
        progress_bar(0.0..=100.0, window.percent as f32),
        text(format!("剩余 {:.1}", window.remaining)).size(12)
    ]
    .spacing(8)
    .into()
}

fn docked(state: &UsageState) -> Element<'_, Message> {
    let status = highest(state).map_or_else(
        || "● 等待数据".to_owned(),
        |window| format!("● {} {:.1}%", window.label, window.percent),
    );
    row![
        text(status).width(Fill),
        button("展开").on_press(Message::FloatModeChanged(FloatMode::Full)),
        button("×").on_press(Message::CloseFloat)
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
    Column::new()
        .push(
            row![
                text(window.label.clone()).width(Fill),
                text(format!("{:.1}%", window.percent))
            ]
            .spacing(8),
        )
        .push(progress_bar(0.0..=100.0, window.percent as f32))
        .spacing(3)
        .into()
}
