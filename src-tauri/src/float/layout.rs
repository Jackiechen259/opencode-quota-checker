//! Pure geometry for the floating window.
//!
//! Everything in this module is deterministic and independent of Tauri, so
//! the native window sizes, the docking decision and the position clamp can
//! be unit-tested on every platform. The constants are the single source of
//! truth for the native window; `src/styles/float-window.css` renders fluidly
//! inside them, and its radius tokens must match [`corner_radius`].

use crate::float::state::FloatPresentation;
use opencode_core::UsageReport;

/// Width of the floating widget in logical pixels, shared by every mode.
pub const FLOAT_WIDTH: f64 = 392.0;
/// Compact card height (the hero row flexes to fill the extra space).
pub const COMPACT_HEIGHT: f64 = 168.0;
/// Docked strip height.
pub const DOCKED_HEIGHT: f64 = 48.0;
/// Full mode lower bound: never shrink below the skeleton/empty layout.
pub const FULL_MIN_HEIGHT: f64 = 280.0;
/// Full mode upper bound: beyond this the quota list scrolls.
pub const FULL_MAX_HEIGHT: f64 = 560.0;
/// One quota card plus its gap, in logical pixels.
pub const FULL_CARD_STEP: f64 = 96.0;
/// Fixed Full chrome (padding + header + gaps + meta + footer), logical px.
pub const FULL_CHROME_HEIGHT: f64 = 134.0;
/// Distance from the monitor top that activates the compact dock.
pub const TOP_SNAP_DISTANCE: f64 = 18.0;
/// Larger release distance prevents repeated mode changes near the boundary.
pub const TOP_RELEASE_DISTANCE: f64 = 24.0;
/// Skeleton size used before the first usage report arrives.
pub const SKELETON_CARD_COUNT: usize = 2;

/// Number of quota cards that drive the Full height.
///
/// No report yet means the loading skeleton is sized for two cards so the
/// window does not resize out from under the loading UI. A report with zero
/// windows still reserves one card.
pub fn quota_count(report: Option<&UsageReport>) -> usize {
    report.map_or(SKELETON_CARD_COUNT, |report| report.windows.len().max(1))
}

/// Ideal Full height for `quota_count` cards, clamped to the window bounds.
pub fn full_height(quota_count: usize) -> f64 {
    (FULL_CHROME_HEIGHT + FULL_CARD_STEP * quota_count as f64)
        .clamp(FULL_MIN_HEIGHT, FULL_MAX_HEIGHT)
}

/// Logical size for a presentation mode under the current usage state.
pub fn size(presentation: FloatPresentation, quota_count: usize) -> (f64, f64) {
    let height = match presentation {
        FloatPresentation::Full => full_height(quota_count),
        FloatPresentation::Compact => COMPACT_HEIGHT,
        FloatPresentation::Docked => DOCKED_HEIGHT,
    };
    (FLOAT_WIDTH, height)
}

/// Corner radius per presentation mode, in logical pixels.
///
/// Must stay in sync with the CSS `border-radius` in `float-window.css`
/// (`--radius-float` / `--radius-float-docked`); the native region and the
/// rendered card would otherwise show a jagged ring between the two shapes.
pub fn corner_radius(presentation: FloatPresentation) -> f32 {
    match presentation {
        FloatPresentation::Docked => 12.0,
        FloatPresentation::Full | FloatPresentation::Compact => 16.0,
    }
}

/// Docking decision with hysteresis.
///
/// `distance` is `window_top - monitor_top` in physical pixels and
/// `scale_factor` scales both thresholds, matching the persisted Windows
/// behavior.
pub fn is_top_docked(currently_docked: bool, distance: f64, scale_factor: f64) -> bool {
    let scale = scale_factor.max(0.01);
    if distance < -TOP_RELEASE_DISTANCE * scale {
        false
    } else if currently_docked {
        distance <= TOP_RELEASE_DISTANCE * scale
    } else {
        distance <= TOP_SNAP_DISTANCE * scale
    }
}

/// Clamps a physical window position into one monitor's visible bounds.
///
/// Used by the non-Windows restore path; the Windows adapter clamps against
/// the native work area (taskbar-aware) instead.
pub fn clamp_position(
    position: (i32, i32),
    monitor_position: (i32, i32),
    monitor_size: (u32, u32),
    window_size: (u32, u32),
) -> (i32, i32) {
    let maximum_x =
        (monitor_position.0 + monitor_size.0 as i32 - window_size.0 as i32).max(monitor_position.0);
    let maximum_y =
        (monitor_position.1 + monitor_size.1 as i32 - window_size.1 as i32).max(monitor_position.1);
    (
        position.0.clamp(monitor_position.0, maximum_x),
        position.1.clamp(monitor_position.1, maximum_y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencode_core::WindowReport;

    fn window(key: &str) -> WindowReport {
        WindowReport {
            key: key.to_owned(),
            label: key.to_owned(),
            quota: 100.0,
            used: 10.0,
            remaining: 90.0,
            percent: 10.0,
            subscribe_time: 0,
            reset_time: 0,
            reset_in_secs: 0,
            limited: false,
        }
    }

    #[test]
    fn full_height_is_clamped_and_scales_with_card_count() {
        assert_eq!(full_height(0), FULL_MIN_HEIGHT);
        assert_eq!(full_height(1), FULL_MIN_HEIGHT, "one card fits the minimum");
        let two = full_height(2);
        assert!(two > FULL_MIN_HEIGHT);
        assert!(two <= FULL_MAX_HEIGHT);
        let four = full_height(4);
        assert!(four > two, "more cards grow the window up to the cap");
        assert_eq!(full_height(5), FULL_MAX_HEIGHT);
        assert_eq!(full_height(100), FULL_MAX_HEIGHT);
    }

    #[test]
    fn fixed_modes_are_constant() {
        assert_eq!(
            size(FloatPresentation::Compact, 2),
            (FLOAT_WIDTH, COMPACT_HEIGHT)
        );
        assert_eq!(
            size(FloatPresentation::Docked, 2),
            (FLOAT_WIDTH, DOCKED_HEIGHT)
        );
    }

    #[test]
    fn full_size_follows_the_quota_count() {
        assert_eq!(
            size(FloatPresentation::Full, 2),
            (FLOAT_WIDTH, full_height(2))
        );
        assert_eq!(
            size(FloatPresentation::Full, 5).1,
            FULL_MAX_HEIGHT,
            "five cards already hit the cap"
        );
    }

    #[test]
    fn quota_count_uses_a_skeleton_and_at_least_one_card() {
        assert_eq!(quota_count(None), SKELETON_CARD_COUNT);

        let empty = UsageReport {
            plan_type: "pro".to_owned(),
            fetched_at: 0,
            windows: Vec::new(),
        };
        assert_eq!(quota_count(Some(&empty)), 1);

        let report = UsageReport {
            plan_type: "pro".to_owned(),
            fetched_at: 0,
            windows: vec![window("a"), window("b"), window("c"), window("d")],
        };
        assert_eq!(quota_count(Some(&report)), 4);
    }

    #[test]
    fn corner_radius_matches_the_css_tokens() {
        assert_eq!(corner_radius(FloatPresentation::Full), 16.0);
        assert_eq!(corner_radius(FloatPresentation::Compact), 16.0);
        assert_eq!(corner_radius(FloatPresentation::Docked), 12.0);
    }

    #[test]
    fn top_snap_uses_hysteresis() {
        assert!(is_top_docked(false, 18.0, 1.0));
        assert!(!is_top_docked(false, 19.0, 1.0));
        assert!(is_top_docked(true, 24.0, 1.0));
        assert!(!is_top_docked(true, 25.0, 1.0));
        assert!(!is_top_docked(false, -25.0, 1.0));
        assert!(
            is_top_docked(true, -20.0, 1.0),
            "still inside the release band above the work area"
        );
    }

    #[test]
    fn top_snap_threshold_scales_with_monitor_dpi() {
        assert!(is_top_docked(false, 27.0, 1.5));
        assert!(!is_top_docked(false, 28.0, 1.5));
        assert!(is_top_docked(true, 36.0, 1.5));
        assert!(!is_top_docked(true, 37.0, 1.5));
    }

    #[test]
    fn top_snap_tolerates_a_non_positive_scale_factor() {
        assert!(is_top_docked(false, 0.0, 0.0));
        assert!(!is_top_docked(false, 1.0, 0.0));
    }

    #[test]
    fn clamp_position_keeps_the_window_inside_the_monitor() {
        let monitor_position = (-1920, 0);
        let monitor_size = (1920, 1080);
        let window_size = (392, 280);

        assert_eq!(
            clamp_position((-2500, -100), monitor_position, monitor_size, window_size),
            (-1920, 0)
        );
        assert_eq!(
            clamp_position((-500, 900), monitor_position, monitor_size, window_size),
            (-500, 800)
        );
        assert_eq!(
            clamp_position((-2000, 100), monitor_position, monitor_size, window_size),
            (-1920, 100)
        );
    }
}
