use iced::border;
use iced::theme::Palette;
use iced::widget::{button, container, progress_bar};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

pub mod colors;
pub mod spacing;
pub mod typography;

pub use colors as palette;

/// Builds the modern, bright "OpenCode Quota Checker" theme with a white base tone.
pub fn application() -> Theme {
    Theme::custom(
        "OpenCode Quota Checker",
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

/// Radius scale (small label 8, button 10, card 16, large card 18).
pub mod radius {
    pub const LABEL: f32 = 8.0;
    pub const BUTTON: f32 = 10.0;
    pub const CARD: f32 = 14.0;
    pub const LARGE_CARD: f32 = 14.0;
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
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(15, 23, 42, 0.18),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 12.0,
        },
        snap: false,
    }
}

/// Compact brand mark used in the floating-window title area.
pub fn float_logo() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::PRIMARY)),
        text_color: Some(palette::SURFACE),
        border: Border {
            radius: 9.0.into(),
            ..Border::default()
        },
        shadow: Shadow {
            color: Color::from_rgba8(59, 130, 246, 0.22),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 5.0,
        },
        snap: false,
    }
}

/// Blue metadata badge for the active AFP plan.
pub fn float_plan_badge() -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::PRIMARY_LIGHT)),
        text_color: Some(palette::PRIMARY),
        border: Border {
            color: palette::PRIMARY_BORDER,
            width: 1.0,
            radius: radius::PILL.into(),
        },
        ..container::Style::default()
    }
}

/// Inset quota row with a subtle health-colored border.
pub fn float_quota_panel(accent: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE_HOVER)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border {
            color: Color { a: 0.22, ..accent },
            width: 1.0,
            radius: 11.0.into(),
        },
        ..container::Style::default()
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
        border: Border {
            color: Color::TRANSPARENT,
            width: 1.0,
            radius: radius::BUTTON.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(palette::PRIMARY_LIGHT)),
            text_color: palette::TEXT_PRIMARY,
            border: Border {
                color: Color::from_rgba8(59, 130, 246, 0.28),
                ..base.border
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(palette::PRIMARY_PRESSED)),
            text_color: palette::PRIMARY_HOVER,
            border: Border {
                color: palette::PRIMARY,
                ..base.border
            },
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: palette::TEXT_MUTED,
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..base
        },
        _ => base,
    }
}

pub fn icon_button_with_focus(
    theme: &Theme,
    status: button::Status,
    focused: bool,
) -> button::Style {
    let mut style = icon_button(theme, status);
    if focused {
        style.border = Border {
            color: palette::PRIMARY,
            width: 2.0,
            radius: radius::BUTTON.into(),
        };
    }
    style
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
