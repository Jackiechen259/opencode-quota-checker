use crate::message::Message;
use crate::theme;
use crate::view::components::card;
use crate::view::components::quota_ring::QuotaRing;
use crate::view::components::status_badge::{self, Tone};
use crate::view::format;
use iced::widget::{canvas, column, container, row, rule, text};
use iced::{Color, Element, Fill, Font, Length};
use volc_core::WindowReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaHealth {
    Healthy,
    Warning,
    Critical,
}

impl QuotaHealth {
    pub fn from_percent(percent: f64) -> Self {
        if percent >= 90.0 {
            Self::Critical
        } else if percent >= 70.0 {
            Self::Warning
        } else {
            Self::Healthy
        }
    }

    fn badge_label(self) -> &'static str {
        match self {
            Self::Healthy => "健康",
            Self::Warning => "接近阈值",
            Self::Critical => "危险",
        }
    }

    fn badge_tone(self) -> Tone {
        match self {
            Self::Healthy => Tone::Success,
            Self::Warning => Tone::Warning,
            Self::Critical => Tone::Danger,
        }
    }

    pub fn progress_color(self) -> Color {
        match self {
            Self::Healthy => theme::palette::PRIMARY,
            Self::Warning => theme::palette::WARNING,
            Self::Critical => theme::palette::DANGER,
        }
    }

    pub fn status_color(self) -> Color {
        match self {
            Self::Healthy => theme::palette::SUCCESS,
            Self::Warning => theme::palette::WARNING,
            Self::Critical => theme::palette::DANGER,
        }
    }

    pub fn color(self) -> Color {
        self.status_color()
    }

    fn status_label(self) -> &'static str {
        match self {
            Self::Healthy => "正常",
            Self::Warning => "警告",
            Self::Critical => "危险",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardLayout {
    Horizontal,
    Stacked,
}

impl CardLayout {
    pub fn height(self) -> f32 {
        match self {
            Self::Horizontal => 240.0,
            Self::Stacked => 342.0,
        }
    }
}

pub fn view(
    window: WindowReport,
    now_ms: i64,
    layout: CardLayout,
    width: f32,
) -> Element<'static, Message> {
    let health = QuotaHealth::from_percent(window.percent);
    let reset_seconds = window.reset_time.saturating_sub(now_ms) / 1_000;

    let header = row![
        status(health),
        text(window.label)
            .size(theme::typography::CARD_TITLE)
            .color(theme::palette::TEXT_PRIMARY)
            .width(Fill),
        status_badge::view(health.badge_label(), health.badge_tone()),
    ]
    .spacing(theme::spacing::SM)
    .align_y(iced::Alignment::Center);

    let ring = container(
        canvas(QuotaRing::new(window.percent as f32, health))
            .width(Length::Fixed(112.0))
            .height(Length::Fixed(112.0)),
    )
    .width(Length::Shrink)
    .align_x(iced::alignment::Horizontal::Center);

    let metric_size = if width < 520.0 {
        16.0
    } else {
        theme::typography::METRIC_VALUE
    };
    let metrics = row![
        metric("已用", format::number(window.used), metric_size),
        metric("总额", format::number(window.quota), metric_size),
        metric("剩余", format::number(window.remaining), metric_size),
    ]
    .spacing(theme::spacing::MD)
    .width(Fill);

    let body: Element<'static, Message> = match layout {
        CardLayout::Horizontal => row![ring, metrics]
            .spacing(theme::spacing::XL)
            .align_y(iced::Alignment::Center)
            .width(Fill)
            .into(),
        CardLayout::Stacked => column![ring, metrics]
            .spacing(theme::spacing::BASE)
            .align_x(iced::Alignment::Center)
            .width(Fill)
            .into(),
    };

    let reset = row![
        text("下次重置")
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_MUTED)
            .width(Fill),
        text(format::countdown(reset_seconds))
            .size(theme::typography::BODY)
            .color(theme::palette::TEXT_PRIMARY),
    ]
    .align_y(iced::Alignment::Center);

    card::sized(
        column![header, body, rule::horizontal(1).style(rule_style), reset]
            .spacing(theme::spacing::MD),
        width,
        layout.height(),
    )
    .into()
}

fn status(health: QuotaHealth) -> Element<'static, Message> {
    row![
        text("●").size(10).color(health.status_color()),
        text(health.status_label())
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_SECONDARY),
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center)
    .into()
}

fn metric(label: &'static str, value: String, value_size: f32) -> Element<'static, Message> {
    column![
        text(label)
            .size(theme::typography::LABEL)
            .color(theme::palette::TEXT_MUTED),
        text(value)
            .font(Font::MONOSPACE)
            .size(value_size)
            .color(theme::palette::TEXT_PRIMARY),
    ]
    .spacing(theme::spacing::XS)
    .width(Fill)
    .into()
}

fn rule_style(_theme: &iced::Theme) -> rule::Style {
    rule::Style {
        color: theme::palette::DIVIDER,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: false,
    }
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
