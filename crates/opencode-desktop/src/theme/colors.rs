use iced::Color;

pub const BACKGROUND: Color = Color::from_rgb8(246, 248, 251);
pub const SURFACE: Color = Color::WHITE;
pub const SURFACE_HOVER: Color = Color::from_rgb8(248, 250, 252);

pub const TEXT_PRIMARY: Color = Color::from_rgb8(15, 23, 42);
pub const TEXT_SECONDARY: Color = Color::from_rgb8(71, 85, 105);
pub const TEXT_MUTED: Color = Color::from_rgb8(148, 163, 184);

pub const BORDER: Color = Color::from_rgb8(226, 232, 240);
pub const DIVIDER: Color = Color::from_rgb8(226, 232, 240);

pub const PRIMARY: Color = Color::from_rgb8(59, 130, 246);
pub const PRIMARY_HOVER: Color = Color::from_rgb8(37, 99, 235);
pub const PRIMARY_LIGHT: Color = Color::from_rgb8(239, 246, 255);
pub const PRIMARY_PRESSED: Color = Color::from_rgb8(219, 234, 254);
pub const PRIMARY_BORDER: Color = Color::from_rgba8(59, 130, 246, 0.24);

pub const SUCCESS: Color = Color::from_rgb8(34, 197, 94);
pub const SUCCESS_LIGHT: Color = Color::from_rgba8(34, 197, 94, 0.08);
pub const SUCCESS_BORDER: Color = Color::from_rgba8(34, 197, 94, 0.22);
pub const WARNING: Color = Color::from_rgb8(245, 158, 11);
pub const WARNING_LIGHT: Color = Color::from_rgba8(245, 158, 11, 0.09);
pub const WARNING_BORDER: Color = Color::from_rgba8(245, 158, 11, 0.24);
pub const DANGER: Color = Color::from_rgb8(239, 68, 68);
pub const DANGER_LIGHT: Color = Color::from_rgba8(239, 68, 68, 0.08);
pub const DANGER_BORDER: Color = Color::from_rgba8(239, 68, 68, 0.22);

pub const TRACK: Color = Color::from_rgb8(226, 232, 240);

/// Title-bar window-control button hover background (neutral gray).
pub const TITLE_BAR_HOVER: Color = Color::from_rgb8(228, 232, 238);
/// Title-bar window-control button pressed background (darker gray).
pub const TITLE_BAR_PRESSED: Color = Color::from_rgb8(210, 216, 226);
/// Title-bar close button hover background (Windows-style red).
pub const TITLE_BAR_CLOSE_HOVER: Color = Color::from_rgb8(196, 43, 28);
/// Title-bar close button pressed background (deeper red).
pub const TITLE_BAR_CLOSE_PRESSED: Color = Color::from_rgb8(158, 31, 19);
