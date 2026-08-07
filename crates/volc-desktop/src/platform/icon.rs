//! Embedded application artwork shared by native windows and the system tray.

const WINDOW_SIZE: u32 = 256;
const TRAY_SIZE: u32 = 32;
const WINDOW_RGBA: &[u8] = include_bytes!("../../../../assets/icons/window.rgba");
const TRAY_RGBA: &[u8] = include_bytes!("../../../../assets/icons/tray.rgba");

/// Returns the full-color icon used by native application windows.
pub fn window() -> iced::window::Icon {
    iced::window::icon::from_rgba(WINDOW_RGBA.to_vec(), WINDOW_SIZE, WINDOW_SIZE)
        .expect("embedded window icon must contain 256x256 RGBA pixels")
}

/// Returns the small icon used by the native system tray.
pub fn tray() -> Result<tray_icon::Icon, String> {
    tray_icon::Icon::from_rgba(TRAY_RGBA.to_vec(), TRAY_SIZE, TRAY_SIZE)
        .map_err(|error| error.to_string())
}
