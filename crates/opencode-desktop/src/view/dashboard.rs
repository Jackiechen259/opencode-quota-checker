use crate::message::Message;
use crate::state::UsageState;
use crate::theme;
use crate::view::components::loading_spinner::LoadingSpinner;
use crate::view::components::notice::{self, NoticeKind};
use crate::view::components::quota_card::{self, CardLayout};
use crate::view::components::section_header;
use crate::view::components::{card, icons, settings};
use crate::view::overview;
use iced::alignment::Horizontal;
use iced::widget::{button, canvas, column, container, responsive, space, text, Column, Row};
use iced::{Element, Fill, FillPortion, Length};
use opencode_core::{UsageReport, WindowReport};

pub fn view(state: &UsageState) -> Element<'_, Message> {
    let report = state.report.clone();
    let error = state.error.clone();
    let now_ms = state.now_ms;
    let loading = state.loading;

    responsive(move |size| {
        let mut content = column![].spacing(theme::spacing::SECTION_GAP);

        if let Some(report) = report.clone() {
            content = content.push(overview::view(report.clone(), now_ms));
            content = content.push(details(report, now_ms, size.width));
        } else if loading {
            content = content.push(skeleton(now_ms));
        } else {
            content = content.push(empty_state());
        }

        if let Some(error) = error.clone() {
            content = content.push(notice::view(
                NoticeKind::Warning,
                &format!("暂时无法更新配额数据：{}", error.user),
            ));
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
    let header = section_header::with_icon(
        icons::LAYERS,
        "详细指标",
        "各配额窗口明细",
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
    // Coarse proxy for the old per-card width threshold (~480px-wide cards
    // were roomy enough for the horizontal layout).
    let layout = if columns == 1 || (columns == 2 && window_width >= 1_032.0) {
        CardLayout::Horizontal
    } else {
        CardLayout::Stacked
    };

    let mut grid = Column::new().spacing(theme::spacing::CARD_GAP);
    for chunk in windows.chunks(columns) {
        let mut row = Row::new().spacing(theme::spacing::CARD_GAP).width(Fill);
        for window in chunk {
            row = row.push(quota_card::view(window.clone(), now_ms, layout));
        }
        // Pad short rows with equal-portion spacers so every row shares the
        // same column widths.
        for _ in chunk.len()..columns {
            row = row.push(space::horizontal().width(FillPortion(1)));
        }
        grid = grid.push(row);
    }
    grid.into()
}

fn skeleton(now_ms: i64) -> Element<'static, Message> {
    card::standard(
        column![
            canvas(LoadingSpinner::new(now_ms))
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(48.0)),
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

fn empty_state() -> Element<'static, Message> {
    card::standard(
        column![
            settings::icon_tile(icons::ALERT),
            text("还没有配额数据")
                .size(theme::typography::CARD_TITLE)
                .color(theme::palette::TEXT_PRIMARY),
            text("点击下方按钮立即刷新，或等待自动检查。")
                .size(theme::typography::LABEL)
                .color(theme::palette::TEXT_MUTED),
            button("立即刷新")
                .on_press(Message::Refresh)
                .style(button::primary)
                .padding([8, 16]),
        ]
        .spacing(theme::spacing::SM)
        .align_x(Horizontal::Center),
    )
    .padding(48)
    .align_x(Horizontal::Center)
    .into()
}
