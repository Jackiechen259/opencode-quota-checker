use iced::border;
use iced::theme::Palette;
use iced::widget::{button, container, progress_bar};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

/// Builds the modern, bright "VOLC Status" theme with a white base tone.
pub fn application() -> Theme {
    Theme::custom(
        "VOLC Status",
        Palette {
            background: palette::BACKGROUND,
            text: palette::TEXT_PRIMARY,
            primary: palette::PRIMARY,
            success: palette::SUCCESS,
            warning: palette::WARNING,
            danger: palette::DANGER,
        },
    )
}

/// Centralized design tokens. No component may hardcode colors.
#[allow(dead_code)]
pub mod palette {
    use iced::Color;

    pub const BACKGROUND: Color = Color::from_rgb8(244, 247, 251);
    pub const SURFACE: Color = Color::WHITE;
    pub const SURFACE_HOVER: Color = Color::from_rgb8(248, 250, 252);

    pub const TEXT_PRIMARY: Color = Color::from_rgb8(15, 23, 42);
    pub const TEXT_SECONDARY: Color = Color::from_rgb8(71, 85, 105);
    pub const TEXT_MUTED: Color = Color::from_rgb8(148, 163, 184);

    pub const BORDER: Color = Color::from_rgb8(226, 232, 240);
    pub const DIVIDER: Color = Color::from_rgb8(232, 237, 243);

    pub const PRIMARY: Color = Color::from_rgb8(59, 130, 246);
    pub const PRIMARY_HOVER: Color = Color::from_rgb8(37, 99, 235);

    pub const SUCCESS: Color = Color::from_rgb8(16, 185, 129);
    pub const WARNING: Color = Color::from_rgb8(245, 158, 11);
    pub const DANGER: Color = Color::from_rgb8(239, 68, 68);

    /// Progress-bar / ring background track.
    pub const TRACK: Color = Color::from_rgb8(228, 234, 242);
}

#[allow(dead_code)]
pub mod spacing {
    pub const XS: u16 = 4;
    pub const SM: u16 = 8;
    pub const MD: u16 = 12;
    pub const BASE: u16 = 16;
    pub const LG: u16 = 20;
    pub const XL: u16 = 24;
    pub const XXL: u16 = 32;
    pub const HUGE: u16 = 40;
}

/// Radius scale (small label 8, button 10, card 16, large card 18).
pub mod radius {
    pub const LABEL: f32 = 8.0;
    pub const BUTTON: f32 = 10.0;
    pub const CARD: f32 = 16.0;
    pub const LARGE_CARD: f32 = 18.0;
    pub const PILL: f32 = 999.0;
}

/// Soft shadow shared by elevated cards: `0 2px 8px rgba(15,23,42,0.06)`.
pub const CARD_SHADOW: Shadow = Shadow {
    color: Color::from_rgba8(15, 23, 42, 0.06),
    offset: Vector::new(0.0, 2.0),
    blur_radius: 8.0,
};

/// Standard card: white surface, hairline border, light shadow.
pub fn card() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: radius::CARD.into(),
        },
        shadow: CARD_SHADOW,
        snap: false,
    }
}

/// Large overview card.
pub fn large_card() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: radius::LARGE_CARD.into(),
        },
        shadow: CARD_SHADOW,
        snap: false,
    }
}

/// Flat inset panel: muted surface, hairline border, no shadow.
pub fn panel() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE_HOVER)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: radius::CARD.into(),
        },
        ..container::Style::default()
    }
}

/// Header surface: pure white with a bottom hairline.
pub fn header_surface() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: palette::BORDER,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(15, 23, 42, 0.04),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 2.0,
        },
        snap: false,
    }
}

/// Page background wrapper.
pub fn page_background() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BACKGROUND)),
        ..container::Style::default()
    }
}

/// Floating-window surface: white with a crisp border.
pub fn float_card() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: radius::CARD.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(15, 23, 42, 0.12),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        snap: false,
    }
}

/// Semi-transparent backdrop for modal dialogs.
pub fn backdrop() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(15, 23, 42, 0.45))),
        ..container::Style::default()
    }
}

/// Dialog surface: elevated white card.
pub fn dialog_surface() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: radius::LARGE_CARD.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(15, 23, 42, 0.20),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        snap: false,
    }
}

/// Danger-tinted notice box.
pub fn danger_box() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(239, 68, 68, 0.08))),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: Color::from_rgba8(239, 68, 68, 0.30),
            width: 1.0,
            radius: radius::CARD.into(),
        },
        ..container::Style::default()
    }
}

/// Warning-tinted notice box.
pub fn warning_box() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(245, 158, 11, 0.10))),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: Color::from_rgba8(245, 158, 11, 0.35),
            width: 1.0,
            radius: radius::CARD.into(),
        },
        ..container::Style::default()
    }
}

/// Primary-tinted toast.
pub fn toast() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(59, 130, 246, 0.10))),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: Color::from_rgba8(59, 130, 246, 0.30),
            width: 1.0,
            radius: radius::CARD.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(15, 23, 42, 0.08),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        snap: false,
    }
}

/// Tooltip surface.
pub fn tooltip_surface() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::TEXT_PRIMARY)),
        text_color: Some(palette::SURFACE),
        border: Border {
            color: palette::TEXT_PRIMARY,
            width: 0.0,
            radius: radius::LABEL.into(),
        },
        ..container::Style::default()
    }
}

/// Pill progress bar styled with the given bar color.
pub fn progress_style(bar_color: Color) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(palette::TRACK),
        bar: Background::Color(bar_color),
        border: border::rounded(radius::PILL),
    }
}

/// Ghost icon button: transparent until hovered, then a soft pill.
pub fn icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: palette::TEXT_SECONDARY,
        border: border::rounded(radius::BUTTON),
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(palette::SURFACE_HOVER)),
            text_color: palette::TEXT_PRIMARY,
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: palette::TEXT_MUTED,
            ..base
        },
        _ => base,
    }
}

/// Soft secondary button (e.g. text nav actions).
pub fn soft_button(_theme: &Theme, status: button::Status) -> button::Style {
    icon_button(_theme, status)
}

/// The blue rounded logo block content color.
pub fn logo() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::PRIMARY)),
        text_color: Some(palette::SURFACE),
        border: Border {
            color: palette::PRIMARY,
            width: 0.0,
            radius: radius::LABEL.into(),
        },
        ..container::Style::default()
    }
}
