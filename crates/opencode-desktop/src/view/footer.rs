use crate::message::Message;
use crate::theme;
use crate::view::format;
use iced::widget::{column, container, rule, text};
use iced::{Element, Fill, Length};
use opencode_core::UsageReport;

/// Footer with the absolute timestamp of the last successful refresh; the
/// relative time and refresh state live in the control bar.
pub fn view(report: Option<&UsageReport>) -> Element<'static, Message> {
    let label = report.map_or_else(
        || "最后更新：暂无数据".to_owned(),
        |report| format!("最后更新：{}", format::timestamp(report.fetched_at)),
    );

    column![
        rule::horizontal(1),
        container(
            text(label)
                .size(theme::typography::CAPTION)
                .color(theme::palette::TEXT_MUTED),
        )
        .width(Fill)
        .height(Length::Fixed(34.0))
        .padding([9.0, theme::spacing::PAGE_PADDING]),
    ]
    .spacing(0)
    .into()
}
