use crate::theme;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{Point, Rectangle, Renderer, Theme};
use std::f32::consts::{FRAC_PI_2, PI};

pub struct LoadingSpinner {
    phase: f32,
}

impl LoadingSpinner {
    pub fn new(now_ms: i64) -> Self {
        Self {
            phase: (now_ms.rem_euclid(1_000) as f32 / 1_000.0) * 2.0 * PI,
        }
    }
}

impl<Message> canvas::Program<Message> for LoadingSpinner {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = frame.center();
        let radius = frame.width().min(frame.height()) * 0.34;
        let path = Path::new(|builder| {
            let start = self.phase - FRAC_PI_2;
            builder.move_to(Point::new(
                center.x + radius * start.cos(),
                center.y + radius * start.sin(),
            ));
            builder.arc(canvas::path::Arc {
                center,
                radius,
                start_angle: iced::Radians(start),
                end_angle: iced::Radians(start + PI * 1.45),
            });
        });
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(theme::palette::PRIMARY)
                .with_width(2.0)
                .with_line_cap(canvas::LineCap::Round),
        );
        vec![frame.into_geometry()]
    }
}
