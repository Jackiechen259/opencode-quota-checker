use iced::{window, Size, Task};

/// Opens the single main window with close requests routed through `Message`.
pub fn open() -> (window::Id, Task<window::Id>) {
    window::open(window::Settings {
        size: Size::new(1_040.0, 720.0),
        min_size: Some(Size::new(760.0, 560.0)),
        exit_on_close_request: false,
        ..window::Settings::default()
    })
}
