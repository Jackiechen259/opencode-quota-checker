//! Single source of truth for the application icon rendered inside the UI.
//!
//! The title bar loads the icon through this component, so the brand asset
//! is embedded exactly once.

use crate::message::Message;
use iced::widget::image::{self, Handle, Image};
use iced::{Element, Length};

/// The official application icon (512×512 PNG with transparent rounded
/// corners); same brand source as `window.rgba` / `tray.rgba`.
const SOURCE: &[u8] = include_bytes!("../../../../../assets/icons/icon.png");

static HANDLE: std::sync::OnceLock<Handle> = std::sync::OnceLock::new();

/// Renders the official application icon at the given square size.
pub fn view(size: f32) -> Element<'static, Message> {
    Image::new(
        HANDLE
            .get_or_init(|| image::Handle::from_bytes(SOURCE))
            .clone(),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .into()
}
