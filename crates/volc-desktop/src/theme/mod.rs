use iced::theme::Palette;
use iced::{Color, Theme};

/// Builds the shared high-contrast VOLC Status theme.
pub fn application() -> Theme {
    Theme::custom(
        "VOLC Status",
        Palette {
            background: Color::from_rgb8(0x0f, 0x14, 0x20),
            text: Color::from_rgb8(0xec, 0xf1, 0xfa),
            primary: Color::from_rgb8(0x4b, 0x8d, 0xff),
            success: Color::from_rgb8(0x52, 0xd2, 0x73),
            warning: Color::from_rgb8(0xf2, 0xc1, 0x4e),
            danger: Color::from_rgb8(0xff, 0x6b, 0x6b),
        },
    )
}
