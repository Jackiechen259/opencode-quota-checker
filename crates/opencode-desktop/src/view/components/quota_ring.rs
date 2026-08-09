//! A lightweight circular progress indicator drawn on an Iced `Canvas`.

use iced::mouse;
use iced::widget::canvas::{self, Cache, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme};
use std::f32::consts::FRAC_PI_2;

use crate::theme;
use crate::view::components::quota_card::QuotaHealth;

/// Visual configuration for a [`QuotaRing`].
#[derive(Debug, Clone)]
struct RingSpec {
    percent: f32,
    color: Color,
    label: String,
    text_color: Color,
}

/// A canvas program rendering a quota progress ring with a centered percent.
pub struct QuotaRing {
    spec: RingSpec,
    cache: Cache,
}

impl QuotaRing {
    /// Creates a ring of the given health, drawing up to `percent` (clamped
    /// for the arc; the label shows the real value when over 100%).
    pub fn new(percent: f32, health: QuotaHealth) -> Self {
        let label = format!("{}%", percent.round() as i32);
        let mut spec = RingSpec {
            percent,
            color: health.progress_color(),
            label,
            text_color: theme::palette::TEXT_PRIMARY,
        };
        if !percent.is_finite() || percent < 0.0 {
            spec.percent = 0.0;
            spec.label = "-".to_owned();
            spec.color = theme::palette::TEXT_MUTED;
        }
        Self {
            spec,
            cache: Cache::default(),
        }
    }
}

impl<Message> canvas::Program<Message> for QuotaRing {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            draw_ring(frame, &self.spec);
        });
        vec![geometry]
    }
}

fn draw_ring(frame: &mut Frame, spec: &RingSpec) {
    let center = frame.center();
    let size = frame.width().min(frame.height());
    let stroke_width = 10.0;
    let radius = (size - stroke_width) / 2.0 - 1.0;

    // Background track.
    let track = Path::circle(center, radius);
    frame.stroke(
        &track,
        Stroke::default()
            .with_color(theme::palette::TRACK)
            .with_width(stroke_width),
    );

    // Progress arc, clamped to a full ring at/over 100%.
    let ratio = (spec.percent / 100.0).clamp(0.0, 1.0);
    let sweep = ratio * 2.0 * std::f32::consts::PI;
    if sweep > 1e-3 {
        let start = -FRAC_PI_2; // 12 o'clock.
        let arc_path = if ratio >= 1.0 {
            Path::circle(center, radius)
        } else {
            Path::new(|b| {
                let start_point = Point::new(
                    center.x + radius * start.cos(),
                    center.y + radius * start.sin(),
                );
                b.move_to(start_point);
                b.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: iced::Radians(start),
                    end_angle: iced::Radians(start + sweep),
                });
            })
        };
        frame.stroke(
            &arc_path,
            Stroke::default()
                .with_color(spec.color)
                .with_width(stroke_width)
                .with_line_cap(iced::widget::canvas::LineCap::Round),
        );
    }

    // Center percent label.
    frame.fill_text(canvas::Text {
        content: spec.label.clone(),
        position: center,
        color: spec.text_color,
        size: iced::Pixels(theme::typography::RING_VALUE),
        font: crate::theme::typography::ui_semibold(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        ..canvas::Text::default()
    });
}
