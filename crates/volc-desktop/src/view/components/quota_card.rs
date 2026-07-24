use crate::message::Message;
use crate::theme;
use crate::view::components::quota_ring::QuotaRing;
use crate::view::format;
use iced::widget::{canvas, column, container, row, rule, text};
use iced::{Color, Element, Fill};
use volc_core::WindowReport;

/// Semantic quota health independent of widget construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaHealth {
    /// Below 70% usage.
    Healthy,
    /// At least 70% but below 90%.
    Warning,
    /// At least 90% usage.
    Critical,
}

impl QuotaHealth {
    /// Derives health from a bounded usage percentage.
    pub fn from_percent(percent: f64) -> Self {
        if percent >= 90.0 {
            Self::Critical
        } else if percent >= 70.0 {
            Self::Warning
        } else {
            Self::Healthy
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Healthy => "健康",
            Self::Warning => "接近阈值",
            Self::Critical => "需要关注",
        }
    }

    /// Accent color tuned for a light surface.
    pub fn color(self) -> Color {
        match self {
            Self::Healthy => theme::palette::SUCCESS,
            Self::Warning => theme::palette::WARNING,
            Self::Critical => theme::palette::DANGER,
        }
    }

    fn dot_text(self) -> &'static str {
        match self {
            Self::Healthy => "正常",
            Self::Warning => "告警",
            Self::Critical => "危险",
        }
    }
}

/// Layout hint for a quota card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardLayout {
    /// `[ring] [info]` side by side.
    Wide,
    /// `[ring]` over `[info]`.
    Narrow,
}

/// Renders one display-ready quota window.
pub fn view(window: WindowReport, now_ms: i64, layout: CardLayout) -> Element<'static, Message> {
    let health = QuotaHealth::from_percent(window.percent);
    let reset_seconds = window.reset_time.saturating_sub(now_ms) / 1_000;

    let header = row![
        status_dot(health),
        text(window.label)
            .size(18)
            .color(theme::palette::TEXT_PRIMARY),
        text(health.label()).size(12).color(health.color()),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let ring = canvas(QuotaRing::new(window.percent as f32, health))
        .width(iced::Length::Fixed(132.0))
        .height(iced::Length::Fixed(132.0));

    let info = column![
        row![
            metric("已用", format!("{:.1}", window.used)),
            metric("总额", format!("{:.1}", window.quota)),
            metric("剩余", format!("{:.1}", window.remaining)),
        ]
        .spacing(16),
        rule::horizontal(1),
        text(format!("下次重置：{}", format::countdown(reset_seconds)))
            .size(13)
            .color(theme::palette::TEXT_SECONDARY),
    ]
    .spacing(12);

    let body: Element<'_, Message> = match layout {
        CardLayout::Wide => row![ring, info]
            .spacing(20)
            .align_y(iced::Alignment::Center)
            .into(),
        CardLayout::Narrow => column![ring, info]
            .spacing(16)
            .align_x(iced::Alignment::Center)
            .into(),
    };

    container(column![header, body].spacing(16))
        .width(Fill)
        .padding(24)
        .style(move |_| theme::card())
        .into()
}

/// A small colored dot with a status word.
fn status_dot(health: QuotaHealth) -> Element<'static, Message> {
    row![
        text("●").size(12).color(health.color()),
        text(health.dot_text())
            .size(12)
            .color(theme::palette::TEXT_SECONDARY),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .into()
}

fn metric(label: &'static str, value: String) -> Element<'static, Message> {
    column![
        text(label).size(12).color(theme::palette::TEXT_MUTED),
        text(value).size(20).color(theme::palette::TEXT_PRIMARY),
    ]
    .spacing(4)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_buckets_are_boundaries() {
        assert_eq!(QuotaHealth::from_percent(69.9), QuotaHealth::Healthy);
        assert_eq!(QuotaHealth::from_percent(70.0), QuotaHealth::Warning);
        assert_eq!(QuotaHealth::from_percent(89.9), QuotaHealth::Warning);
        assert_eq!(QuotaHealth::from_percent(90.0), QuotaHealth::Critical);
    }
}
