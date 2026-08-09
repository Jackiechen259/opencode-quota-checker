use crate::config::FloatMode;
use crate::message::Message;
use crate::state::UsageState;
use crate::theme;
use crate::view::components::dot;
use crate::view::components::icon_button;
use crate::view::components::icons;
use crate::view::components::quota_card::QuotaHealth;
use crate::view::format;
use iced::widget::{button, column, container, mouse_area, progress_bar, row, text, Column};
use iced::{Alignment, Element, Fill, Length};
use opencode_core::{UsageReport, WindowReport};

/// Renders the floating window from the shared usage state.
pub fn view(state: &UsageState, mode: FloatMode) -> Element<'_, Message> {
    let (margin, padding) = match mode {
        FloatMode::Full => (7.0, [12.0, 13.0]),
        FloatMode::Compact => (6.0, [9.0, 11.0]),
        FloatMode::Docked => (4.0, [5.0, 8.0]),
    };
    let content = match mode {
        FloatMode::Full => full(state),
        FloatMode::Compact => compact(state),
        FloatMode::Docked => docked(state),
    };

    container(
        container(content)
            .width(Fill)
            .height(Fill)
            .padding(padding)
            .style(move |_| theme::float_card()),
    )
    .width(Fill)
    .height(Fill)
    .padding(margin)
    .into()
}

fn full(state: &UsageState) -> Element<'_, Message> {
    let mut content = column![header(state, FloatMode::Full)].spacing(10);

    if let Some(report) = &state.report {
        let highest_percent = highest(state).map_or(0.0, |window| window.percent);
        content = content.push(
            row![
                container(
                    text(plan_label(report))
                        .size(11)
                        .color(theme::palette::PRIMARY),
                )
                .padding([4, 8])
                .style(move |_| theme::float_plan_badge()),
                text(format!("共 {} 个配额周期", report.windows.len()))
                    .size(11)
                    .color(theme::palette::TEXT_MUTED)
                    .width(Fill),
                text(format!("最高 {:.1}%", highest_percent))
                    .size(11)
                    .color(QuotaHealth::from_percent(highest_percent).color()),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );

        for window in &report.windows {
            content = content.push(window_card(window, state.now_ms));
        }

        content = content.push(
            row![
                text("自动监测中")
                    .size(10)
                    .color(theme::palette::TEXT_MUTED)
                    .width(Fill),
                text(format!(
                    "更新于 {}",
                    format::relative(report.fetched_at, state.now_ms)
                ))
                .size(10)
                .color(theme::palette::TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
        );
    } else {
        content = content.push(empty_state(state));
    }

    content.into()
}

fn compact(state: &UsageState) -> Element<'_, Message> {
    let mut content = column![header(state, FloatMode::Compact)].spacing(8);
    let Some(window) = highest(state) else {
        return content.push(empty_state(state)).into();
    };

    let health = QuotaHealth::from_percent(window.percent);
    let progress_color = health.progress_color();
    content = content
        .push(
            row![
                column![
                    text(&window.label)
                        .size(12)
                        .color(theme::palette::TEXT_SECONDARY),
                    text(format!("{:.1}%", window.percent))
                        .font(theme::typography::ui_semibold())
                        .size(23)
                        .color(health.color()),
                ]
                .spacing(1)
                .width(Fill),
                column![
                    text("剩余额度").size(10).color(theme::palette::TEXT_MUTED),
                    text(format::number(window.remaining))
                        .font(theme::typography::ui_semibold())
                        .size(15)
                        .color(theme::palette::TEXT_PRIMARY),
                ]
                .spacing(2)
                .align_x(iced::alignment::Horizontal::Right),
            ]
            .align_y(Alignment::End),
        )
        .push(
            progress_bar(0.0..=100.0, window.percent as f32)
                .girth(7)
                .style(move |_| theme::progress_style(progress_color)),
        )
        .push(
            row![
                text(health_label(health))
                    .size(10)
                    .color(health.status_color())
                    .width(Fill),
                text(format!(
                    "{} 后重置",
                    format::countdown_short(window.reset_time.saturating_sub(state.now_ms) / 1_000)
                ))
                .size(10)
                .color(theme::palette::TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
        );
    content.into()
}

fn docked(state: &UsageState) -> Element<'_, Message> {
    let status: Element<'_, Message> = highest(state).map_or_else(
        || {
            row![
                dot::view(theme::palette::TEXT_MUTED, 9.0),
                text(if state.loading {
                    "正在同步用量…"
                } else {
                    "等待用量数据"
                })
                .size(11)
                .color(theme::palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Fill)
            .into()
        },
        |window| {
            let health = QuotaHealth::from_percent(window.percent);
            let progress_color = health.progress_color();
            column![
                row![
                    dot::view(health.status_color(), 9.0),
                    text(&window.label)
                        .size(11)
                        .color(theme::palette::TEXT_SECONDARY),
                    text(format!("{:.1}%", window.percent))
                        .font(theme::typography::ui_semibold())
                        .size(13)
                        .color(health.color()),
                    text(format!("余 {}", format::number(window.remaining)))
                        .size(10)
                        .color(theme::palette::TEXT_MUTED),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                progress_bar(0.0..=100.0, window.percent as f32)
                    .girth(4)
                    .style(move |_| theme::progress_style(progress_color)),
            ]
            .spacing(3)
            .width(Fill)
            .into()
        },
    );
    let drag_content = mouse_area(container(status).width(Fill))
        .on_press(Message::DragFloat)
        .interaction(iced::mouse::Interaction::Grab);

    row![
        drag_content,
        button("展开")
            .on_press(Message::FloatModeChanged(FloatMode::Full))
            .style(theme::soft_button)
            .padding([3, 6]),
        button("×")
            .on_press(Message::CloseFloat)
            .style(theme::soft_button)
            .padding([3, 7]),
    ]
    .spacing(3)
    .align_y(Alignment::Center)
    .into()
}

fn header(state: &UsageState, mode: FloatMode) -> Element<'_, Message> {
    let (status, status_color) = if state.loading {
        ("同步中", theme::palette::PRIMARY)
    } else if state.error.is_some() {
        ("更新失败", theme::palette::DANGER)
    } else if state.report.is_some() {
        ("监测正常", theme::palette::SUCCESS)
    } else {
        ("等待数据", theme::palette::TEXT_MUTED)
    };

    let brand = mouse_area(
        row![
            container(text("OC").size(11).color(theme::palette::SURFACE))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .style(move |_| theme::float_logo()),
            column![
                text("OpenCode Quota Checker")
                    .size(13)
                    .color(theme::palette::TEXT_PRIMARY),
                row![
                    dot::view(status_color, 8.0),
                    text(status).size(9).color(theme::palette::TEXT_MUTED),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            ]
            .spacing(0),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(Message::DragFloat)
    .interaction(iced::mouse::Interaction::Grab);

    let mode_button = match mode {
        FloatMode::Full => icon_button::view(
            icons::MINIMIZE,
            "切换到精简视图",
            Message::FloatModeChanged(FloatMode::Compact),
            false,
        ),
        FloatMode::Compact => icon_button::view(
            icons::EXPAND,
            "展开全部配额",
            Message::FloatModeChanged(FloatMode::Full),
            false,
        ),
        FloatMode::Docked => unreachable!("docked mode does not use the standard header"),
    };

    row![
        brand,
        container(row![]).width(Fill),
        icon_button::view(icons::REFRESH, "立即刷新", Message::Refresh, false),
        mode_button,
        icon_button::view(icons::CLOSE, "关闭悬浮窗", Message::CloseFloat, false),
    ]
    .spacing(2)
    .align_y(Alignment::Center)
    .into()
}

fn window_card(window: &WindowReport, now_ms: i64) -> Element<'static, Message> {
    let health = QuotaHealth::from_percent(window.percent);
    let accent = health.status_color();
    let progress_color = health.progress_color();
    let reset_seconds = window.reset_time.saturating_sub(now_ms) / 1_000;

    container(
        Column::new()
            .push(
                row![
                    row![
                        dot::view(accent, 9.0),
                        text(window.label.clone())
                            .size(12)
                            .color(theme::palette::TEXT_PRIMARY),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .width(Fill),
                    text(format!("{:.1}%", window.percent))
                        .font(theme::typography::ui_semibold())
                        .size(14)
                        .color(accent),
                ]
                .align_y(Alignment::Center),
            )
            .push(
                progress_bar(0.0..=100.0, window.percent as f32)
                    .girth(6)
                    .style(move |_| theme::progress_style(progress_color)),
            )
            .push(
                row![
                    text(format!(
                        "已用 {} / {}",
                        format::number(window.used),
                        format::number(window.quota)
                    ))
                    .size(10)
                    .color(theme::palette::TEXT_MUTED)
                    .width(Fill),
                    text(format!("{} 后重置", format::countdown_short(reset_seconds)))
                        .size(10)
                        .color(theme::palette::TEXT_MUTED),
                ]
                .align_y(Alignment::Center),
            )
            .spacing(6),
    )
    .width(Fill)
    .padding([8, 10])
    .style(move |_| theme::float_quota_panel(accent))
    .into()
}

fn empty_state(state: &UsageState) -> Element<'_, Message> {
    let (title, detail, color) = if let Some(error) = &state.error {
        (
            "暂时无法更新用量",
            error.user.as_str(),
            theme::palette::DANGER,
        )
    } else if state.loading {
        (
            "正在获取用量",
            "首次同步通常只需几秒",
            theme::palette::PRIMARY,
        )
    } else {
        (
            "还没有用量数据",
            "点击右上角刷新按钮开始同步",
            theme::palette::TEXT_MUTED,
        )
    };

    container(
        column![
            dot::view(color, 13.0),
            text(title).size(13).color(theme::palette::TEXT_PRIMARY),
            text(detail).size(10).color(theme::palette::TEXT_MUTED),
        ]
        .spacing(5)
        .align_x(iced::alignment::Horizontal::Center),
    )
    .width(Fill)
    .height(Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

fn highest(state: &UsageState) -> Option<&WindowReport> {
    state.report.as_ref()?.windows.iter().max_by(|left, right| {
        left.percent
            .partial_cmp(&right.percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn health_label(health: QuotaHealth) -> &'static str {
    match health {
        QuotaHealth::Healthy => "状态健康",
        QuotaHealth::Warning => "接近阈值",
        QuotaHealth::Critical => "已达危险阈值",
    }
}

fn plan_label(report: &UsageReport) -> String {
    if report.plan_type.trim().is_empty() {
        "OpenCode Go".to_owned()
    } else {
        format!("OpenCode Go · {}", report.plan_type)
    }
}
