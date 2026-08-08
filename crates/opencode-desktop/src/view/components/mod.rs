pub mod card;
pub mod confirm_dialog;
pub mod icon_button;
pub mod icons;
pub mod loading_spinner;
pub mod metric;
pub mod progress_bar;
pub mod quota_card;
pub mod quota_ring;
pub mod section_header;
pub mod settings;
pub mod status_badge;

/// Font family name used for headings and numerals. Falls back to the system
/// default sans-serif which ships CJK glyphs on Windows.
pub const TEXT_FONT: &str = crate::theme::typography::UI_FONT;
