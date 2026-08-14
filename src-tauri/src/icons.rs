//! Application artwork shared by the tray and windows.

use tauri::image::Image;

/// Full-color PNG window icon (512x512, from `assets/icons/icon.png`).
pub fn window() -> Image<'static> {
    Image::from_bytes(include_bytes!("../icons/icon.png"))
        .expect("embedded window icon must be a valid PNG")
}

/// Small PNG tray icon (32x32, from `assets/icons/32x32.png`).
pub fn tray() -> Result<Image<'static>, String> {
    Image::from_bytes(include_bytes!("../icons/32x32.png"))
        .map_err(|error| error.to_string())
}
