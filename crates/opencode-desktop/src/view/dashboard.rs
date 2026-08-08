use crate::message::Message;
use crate::state::UsageState;
use crate::theme;
use crate::view::components::card;
use crate::view::components::quota_card::{self, CardLayout};
use crate::view::components::section_header;
use crate::view::overview;
use iced::alignment::Horizontal;
use iced::widget::{column, container, responsive, text, Column, Row};
use iced::{Element, Fill, Length};
use opencode_core::{UsageReport, WindowReport};

pub fn view(state: &UsageState) -> Element<'_, Message> {
    let report = state.report.clone();
    let error = state.error.clone();
    let now_ms = state.now_ms;
    let loading = state.loading;

    responsive(move |size| {
        let mut content = column![].spacing(theme::spacing::SECTION_GAP);

        if let Some(report) = report.clone() {
            content = content.push(overview::view(report.clone(), now_ms, loading, size.width));
            content = content.push(details(report, now_ms, size.width));
        } else if loading {
            content = content.push(skeleton());
        } else {
            content = content.push(empty_state("暂无可显示的配额窗口，请尝试刷新。"));
        }

        if let Some(error) = error.clone() {
            content = content.push(error_notice(error.user, error.detail));
        }

        container(content)
            .width(Fill)
            .padding(theme::spacing::PAGE_PADDING)
            .into()
    })
    .height(Length::Shrink)
    .into()
}

fn details(report: UsageReport, now_ms: i64, window_width: f32) -> Element<'static, Message> {
    let header = section_header::view(
        "详细指标",
        text(format!("共 {} 个窗口", report.windows.len()))
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_MUTED)
            .into(),
    );
    let grid = quota_grid(report.windows, now_ms, window_width);
    column![header, grid].spacing(theme::spacing::BASE).into()
}

fn quota_grid(
    windows: Vec<WindowReport>,
    now_ms: i64,
    window_width: f32,
) -> Element<'static, Message> {
    let columns = if window_width > 1_180.0 {
        3usize
    } else if window_width >= 820.0 {
        2usize
    } else {
        1usize
    };
    let available_width = (window_width - theme::spacing::PAGE_PADDING * 2.0).max(240.0);
    let gaps = theme::spacing::CARD_GAP * (columns.saturating_sub(1) as f32);
    let card_width = (available_width - gaps) / columns as f32;
    let layout = if (columns == 1 && card_width >= 560.0) || card_width >= 480.0 {
        CardLayout::Horizontal
    } else {
        CardLayout::Stacked
    };

    let mut grid = Column::new().spacing(theme::spacing::CARD_GAP);
    for chunk in windows.chunks(columns) {
        let cards = chunk
            .iter()
            .cloned()
            .map(|window| quota_card::view(window, now_ms, layout, card_width))
            .collect::<Vec<_>>();
        grid = grid.push(
            Row::with_children(cards)
                .spacing(theme::spacing::CARD_GAP)
                .width(Fill),
        );
    }
    grid.into()
}

fn skeleton() -> Element<'static, Message> {
    card::standard(
        column![
            text("正在安全地加载用量数据…")
                .size(theme::typography::CARD_TITLE)
                .color(theme::palette::TEXT_SECONDARY),
            text("首次加载期间保持页面结构稳定。")
                .size(theme::typography::LABEL)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(theme::spacing::SM)
        .align_x(Horizontal::Center),
    )
    .padding(48)
    .align_x(Horizontal::Center)
    .into()
}

fn empty_state(message: &str) -> Element<'static, Message> {
    card::standard(
        text(message.to_owned())
            .size(theme::typography::CARD_TITLE)
            .color(theme::palette::TEXT_MUTED),
    )
    .padding(48)
    .align_x(Horizontal::Center)
    .into()
}

fn error_notice(user: String, detail: String) -> Element<'static, Message> {
    container(
        column![
            text(format!("暂时无法更新配额数据：{user}"))
                .size(theme::typography::BODY)
                .color(theme::palette::TEXT_PRIMARY),
            text(format!("技术详情：{detail}"))
                .size(theme::typography::LABEL)
                .color(theme::palette::TEXT_MUTED),
        ]
        .spacing(theme::spacing::XS),
    )
    .width(Fill)
    .padding(14)
    .style(move |_| theme::warning_box())
    .into()
}
