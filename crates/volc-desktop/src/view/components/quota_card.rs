use crate::message::Message;
use crate::view::format;
use iced::widget::{column, container, progress_bar, row, text};
use iced::{Background, Border, Color, Element, Fill};
use volc_core::WindowReport;

/// Semantic quota health independent of widget construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaHealth {
    /// Below 80% usage.
    Healthy,
    /// At least 80% but below 90%.
    Warning,
    /// At least 90% usage.
    Critical,
}

impl QuotaHealth {
    /// Derives health from a bounded usage percentage.
    pub fn from_percent(percent: f64) -> Self {
        if percent >= 90.0 {
            Self::Critical
        } else if percent >= 80.0 {
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

    fn color(self) -> Color {
        match self {
            Self::Healthy => Color::from_rgb8(0x52, 0xd2, 0x73),
            Self::Warning => Color::from_rgb8(0xf2, 0xc1, 0x4e),
            Self::Critical => Color::from_rgb8(0xff, 0x6b, 0x6b),
        }
    }
}

/// Renders one display-ready quota window.
pub fn view(window: WindowReport, now_ms: i64) -> Element<'static, Message> {
    let health = QuotaHealth::from_percent(window.percent);
    let reset_seconds = window.reset_time.saturating_sub(now_ms) / 1_000;
    let content = column![
        row![
            text(window.label).size(22),
            text(health.label()).color(health.color())
        ]
        .spacing(12),
        text(format!("{:.1}%", window.percent))
            .size(36)
            .color(health.color()),
        progress_bar(0.0..=100.0, window.percent as f32),
        row![
            metric("已用", format!("{:.1}", window.used)),
            metric("额度", format!("{:.1}", window.quota)),
            metric("剩余", format!("{:.1}", window.remaining)),
        ]
        .spacing(16),
        text(format!("重置倒计时：{}", format::countdown(reset_seconds))).size(13),
    ]
    .spacing(12);

    container(content)
        .width(Fill)
        .padding(18)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0x1b, 0x22, 0x31))),
            border: Border {
                color: health.color(),
                width: 1.0,
                radius: 12.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

fn metric(label: &'static str, value: String) -> Element<'static, Message> {
    column![text(label).size(12), text(value).size(17)]
        .spacing(3)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_boundaries_are_stable() {
        assert_eq!(QuotaHealth::from_percent(79.99), QuotaHealth::Healthy);
        assert_eq!(QuotaHealth::from_percent(80.0), QuotaHealth::Warning);
        assert_eq!(QuotaHealth::from_percent(90.0), QuotaHealth::Critical);
    }
}
