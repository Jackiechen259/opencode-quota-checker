use iced::font::Weight;
use iced::Font;

pub const SECTION_TITLE: f32 = 18.0;
pub const CARD_TITLE: f32 = 16.0;
pub const HERO_VALUE: f32 = 52.0;
pub const RING_VALUE: f32 = 32.0;
pub const METRIC_VALUE: f32 = 17.0;
pub const BODY: f32 = 13.0;
pub const LABEL: f32 = 12.0;
pub const CAPTION: f32 = 11.0;

/// The single packaged font family. Covers simplified Chinese, Latin and
/// numerals so the whole UI renders from one bundled variable font.
pub const FAMILY: &str = "Noto Sans SC";

/// Regular body text weight.
pub fn ui() -> Font {
    Font::with_name(FAMILY)
}

/// Card and section titles.
pub fn ui_medium() -> Font {
    Font {
        weight: Weight::Medium,
        ..Font::with_name(FAMILY)
    }
}

/// Hero numerals and ring labels.
pub fn ui_semibold() -> Font {
    Font {
        weight: Weight::Semibold,
        ..Font::with_name(FAMILY)
    }
}
