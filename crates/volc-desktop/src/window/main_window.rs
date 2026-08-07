use crate::platform::icon;
use iced::{window, Size, Task};

/// Opens the single main window with close requests routed through `Message`.
pub fn open() -> (window::Id, Task<window::Id>) {
    window::open(window::Settings {
        size: Size::new(1_440.0, 900.0),
        min_size: Some(Size::new(680.0, 620.0)),
        icon: Some(icon::window()),
        exit_on_close_request: false,
        ..window::Settings::default()
    })
}
