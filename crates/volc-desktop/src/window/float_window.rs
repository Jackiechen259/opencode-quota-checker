use crate::config::{FloatMode, FloatPosition};
use iced::{window, Point, Size, Task};

/// Opens the unique always-on-top, borderless floating window.
pub fn open(mode: FloatMode, position: Option<FloatPosition>) -> (window::Id, Task<window::Id>) {
    let position = position.map_or(window::Position::Default, |position| {
        window::Position::Specific(Point::new(position.x as f32, position.y as f32))
    });
    window::open(window::Settings {
        size: mode.size(),
        min_size: Some(Size::new(200.0, 40.0)),
        decorations: false,
        transparent: false,
        resizable: true,
        level: window::Level::AlwaysOnTop,
        position,
        exit_on_close_request: false,
        ..window::Settings::default()
    })
}

/// Clamps a floating position to the visible logical bounds of one monitor.
pub fn clamp_position(position: Point, monitor: Size, window_size: Size) -> Point {
    let maximum_x = (monitor.width - window_size.width).max(0.0);
    let maximum_y = (monitor.height - window_size.height).max(0.0);
    Point::new(
        position.x.clamp(0.0, maximum_x),
        position.y.clamp(0.0, maximum_y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offscreen_positions_are_moved_into_view() {
        assert_eq!(
            clamp_position(
                Point::new(-500.0, 2_000.0),
                Size::new(1_920.0, 1_080.0),
                Size::new(344.0, 404.0)
            ),
            Point::new(0.0, 676.0)
        );
    }
}
