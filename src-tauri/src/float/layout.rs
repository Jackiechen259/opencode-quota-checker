//! Pure geometry for the floating window.
//!
//! Everything in this module is deterministic and independent of Tauri, so
//! the native window sizes, the docking decision and the position clamp can
//! be unit-tested on every platform. The constants are the single source of
//! truth for the native window; `src/styles/float-window.css` renders fluidly
//! inside them, and its radius tokens must match [`corner_radius`].

use crate::float::state::{DockPlan, FloatEdge, FloatPresentation};
use opencode_core::UsageReport;

/// Width of the floating widget in logical pixels, shared by every mode.
pub const FLOAT_WIDTH: f64 = 392.0;
/// Compact card height (the hero row flexes to fill the extra space).
pub const COMPACT_HEIGHT: f64 = 168.0;
/// Docked strip height: the top strip is this tall, the side rails this wide.
///
/// A single thin status line, kept as short as the content allows (the close
/// button is 20 px; 1 px outline above and below) so the always-on-top strip
/// covers as little of other windows as possible. The 12 px radius makes the
/// docked surface a capsule at this thickness.
pub const DOCKED_THICKNESS: f64 = 24.0;
/// Narrowest logical length a docked strip may use along its axis.
///
/// The length normally comes from the webview's content measurement (see
/// `docked_size` / `side_docked_size`); this floor only guards against a
/// missing or bad report.
pub const DOCKED_MIN_LENGTH: f64 = 96.0;
/// Full mode lower bound: never shrink below the skeleton/empty layout.
pub const FULL_MIN_HEIGHT: f64 = 280.0;
/// Full mode upper bound: beyond this the quota list scrolls.
pub const FULL_MAX_HEIGHT: f64 = 560.0;
/// Longest side rail, so a long label cannot produce a rail taller than the
/// Full window ever gets.
pub const SIDE_DOCKED_MAX_LENGTH: f64 = FULL_MAX_HEIGHT;
/// Side rail length used before the webview reports its measured content.
pub const SIDE_DOCKED_DEFAULT_LENGTH: f64 = FULL_MIN_HEIGHT;
/// Height of the hover card a docked strip expands into, in logical pixels.
///
/// Hovering a docked strip grows the native window into a small overview card
/// with one ring per quota window. The card keeps [`FLOAT_WIDTH`] so it lines
/// up with the Full/Compact surface, and is tall enough for the header row plus
/// the three rings and their labels.
pub const DOCK_HOVER_HEIGHT: f64 = 176.0;
/// One quota card plus its gap, in logical pixels.
pub const FULL_CARD_STEP: f64 = 96.0;
/// Fixed Full chrome (padding + header + gaps + meta + footer), logical px.
pub const FULL_CHROME_HEIGHT: f64 = 134.0;
/// Distance from a work-area edge that activates docking.
pub const SNAP_DISTANCE: f64 = 18.0;
/// Larger release distance prevents repeated mode changes near the boundary.
pub const RELEASE_DISTANCE: f64 = 24.0;
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
///
/// Docked sizes use the default length; [`docked_size`] / [`side_docked_size`]
/// apply the content-fitted length reported by the webview.
pub fn size(presentation: FloatPresentation, quota_count: usize) -> (f64, f64) {
    match presentation {
        FloatPresentation::Full => (FLOAT_WIDTH, full_height(quota_count)),
        FloatPresentation::Compact => (FLOAT_WIDTH, COMPACT_HEIGHT),
        FloatPresentation::DockedTop => (FLOAT_WIDTH, DOCKED_THICKNESS),
        FloatPresentation::DockedLeft | FloatPresentation::DockedRight => {
            (DOCKED_THICKNESS, SIDE_DOCKED_DEFAULT_LENGTH)
        }
    }
}

/// Clamps a frontend-measured top-strip width into the legal window range.
///
/// A non-finite report falls back to the full width instead of poisoning the
/// native size calculation.
pub fn clamp_docked_width(width: f64) -> f64 {
    if width.is_finite() {
        width.clamp(DOCKED_MIN_LENGTH, FLOAT_WIDTH)
    } else {
        FLOAT_WIDTH
    }
}

/// Native size for the horizontal Docked strip at a measured window width.
///
/// The width comes from the webview (content + chrome) so the snapped strip
/// hugs its content instead of leaving a wide empty band on screen.
pub fn docked_size(width: f64) -> (f64, f64) {
    (clamp_docked_width(width), DOCKED_THICKNESS)
}

/// Clamps a frontend-measured side-rail length into the legal window range.
///
/// The rail is only [`DOCKED_THICKNESS`] wide, so its native *height* is what
/// must be clamped. A non-finite report falls back to the default rail length.
pub fn clamp_side_docked_length(length: f64) -> f64 {
    if length.is_finite() {
        length.clamp(DOCKED_MIN_LENGTH, SIDE_DOCKED_MAX_LENGTH)
    } else {
        SIDE_DOCKED_DEFAULT_LENGTH
    }
}

/// Native size for a vertical side rail at a measured strip length.
pub fn side_docked_size(length: f64) -> (f64, f64) {
    (DOCKED_THICKNESS, clamp_side_docked_length(length))
}

/// Native size of the hover card a docked strip expands into.
///
/// Only a docked window can expand; the card is anchored to the docked edge by
/// [`docked_position`], so a top strip grows downward and a side rail grows
/// away from the screen edge.
pub fn dock_hover_size() -> (f64, f64) {
    (FLOAT_WIDTH, DOCK_HOVER_HEIGHT)
}

/// Corner radius per presentation mode, in logical pixels.
///
/// Must stay in sync with the CSS `border-radius` in `float-window.css`
/// (`--radius-float` / `--radius-float-docked`); the native region and the
/// rendered card would otherwise show a jagged ring between the two shapes.
///
/// A docked strip is a capsule (12 px on a 24 px thickness); its hover card is
/// a real card and shares the Full/Compact radius.
pub fn corner_radius(presentation: FloatPresentation, dock_hover: bool) -> f32 {
    if dock_hover && presentation.docked_edge().is_some() {
        return 16.0;
    }
    match presentation {
        FloatPresentation::Full | FloatPresentation::Compact => 16.0,
        FloatPresentation::DockedTop
        | FloatPresentation::DockedLeft
        | FloatPresentation::DockedRight => 12.0,
    }
}

/// Physical position that keeps a docked window anchored to `edge`.
///
/// `anchor` is the window's coordinate along the edge: `x` for the top strip,
/// `y` for a side rail. It is clamped so the whole window stays inside `work`.
///
/// The collapsed strip and its hover card both use this function: expanding
/// keeps the strip where it was, and collapsing restores that anchor instead
/// of the (possibly clamped) card position, so the strip cannot slide just
/// because the card had to fit on screen.
pub fn docked_position(
    edge: FloatEdge,
    anchor: i32,
    work: (i32, i32, i32, i32),
    size: (u32, u32),
) -> (i32, i32) {
    let (left, top, right, bottom) = work;
    let max_x = (right - size.0 as i32).max(left);
    let max_y = (bottom - size.1 as i32).max(top);
    let anchor_x = anchor.clamp(left, max_x);
    let anchor_y = anchor.clamp(top, max_y);
    match edge {
        FloatEdge::Top => (anchor_x, top),
        FloatEdge::Left => (left, anchor_y),
        FloatEdge::Right => (max_x, anchor_y),
    }
}

/// Signed distances from the window's rectangle to the work-area edges.
///
/// A distance is `0` when the corresponding window edge is flush with the work
/// area, positive when the window is inside, and negative when it has been
/// dragged past the edge. Everything is in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeDistances {
    /// `window.top - work.top`.
    pub top: f64,
    /// `window.left - work.left`.
    pub left: f64,
    /// Distance from the window's right edge to the work-area right edge.
    pub right: f64,
}

impl EdgeDistances {
    /// Measures a window rectangle against a work-area rectangle.
    ///
    /// `work` is `(left, top, right, bottom)`; `size` is the window's outer
    /// size. `i64` math keeps a huge multi-monitor desktop far away from any
    /// overflow.
    pub fn new(position: (i32, i32), size: (u32, u32), work: (i32, i32, i32, i32)) -> Self {
        let x = i64::from(position.0);
        let y = i64::from(position.1);
        let width = i64::from(size.0);
        let (left, top, right, _bottom) = work;
        Self {
            top: (y - i64::from(top)) as f64,
            left: (x - i64::from(left)) as f64,
            right: (i64::from(right) - (x + width)) as f64,
        }
    }

    /// The distance for `edge`.
    pub fn of(&self, edge: FloatEdge) -> f64 {
        match edge {
            FloatEdge::Top => self.top,
            FloatEdge::Left => self.left,
            FloatEdge::Right => self.right,
        }
    }

    /// Whether the edge is inside `band` on both sides of the boundary.
    fn within_band(&self, edge: FloatEdge, band: f64) -> bool {
        let distance = self.of(edge);
        (-band..=band).contains(&distance)
    }

    /// Whether the window is close enough to `edge` to snap to it.
    fn within_snap(&self, edge: FloatEdge, snap: f64, release: f64) -> bool {
        let distance = self.of(edge);
        distance >= -release && distance <= snap
    }

    /// Whether every edge is farther away than the snap band (or has already
    /// been dragged past).
    ///
    /// Used to re-arm docking after an un-dock: a window that is merely
    /// touching a side must leave that side before it can snap again.
    pub fn all_beyond(&self, scale_factor: f64) -> bool {
        let scale = scale_factor.max(0.01);
        let snap = SNAP_DISTANCE * scale;
        let release = RELEASE_DISTANCE * scale;
        [FloatEdge::Top, FloatEdge::Left, FloatEdge::Right]
            .iter()
            .all(|&edge| {
                let distance = self.of(edge);
                distance > snap || distance < -release
            })
    }
}

/// Hysteretic docking decision for one window move.
///
/// `current` is the edge the window is docked to right now, `guarded` blocks
/// new docking until the window leaves every snap band, and `scale_factor`
/// scales both thresholds with the monitor DPI.
///
/// Decision rules:
///
/// 1. A docked window keeps its edge inside the (wider) release band.
/// 2. A top strip that leaves the band un-docks immediately: growing back into
///    the configured layout keeps its top-left corner, so it stays on screen.
/// 3. A side rail that leaves the band keeps the rail and reports `pending`:
///    expanding it while the mouse button is still down would put most of the
///    card off-screen (see `float::controller`).
/// 4. An undocked window docks to the top first (preserving the original
///    top-strip behavior in the corners), otherwise to the nearest side.
/// 5. A guarded window cannot dock until it is clear of every snap band.
pub fn plan_dock(
    current: Option<FloatEdge>,
    guarded: bool,
    distances: EdgeDistances,
    scale_factor: f64,
) -> DockPlan {
    let scale = scale_factor.max(0.01);
    let release = RELEASE_DISTANCE * scale;

    if let Some(edge) = current {
        if distances.within_band(edge, release) {
            return DockPlan::docked(edge);
        }
        return if edge.is_side() {
            DockPlan::pending_undock(edge)
        } else {
            DockPlan::undocked(true)
        };
    }

    if guarded && !distances.all_beyond(scale) {
        return DockPlan::undocked(true);
    }

    match snap_edge(distances, scale) {
        Some(edge) => DockPlan::docked(edge),
        None => DockPlan::undocked(false),
    }
}

/// The edge an undocked window would snap to right now, if any.
///
/// The top edge wins whenever it is within the snap band, which keeps the
/// original long-standing behavior (a window dragged to the top-left corner
/// becomes the top strip, not a left rail). Side edges are otherwise compared
/// by distance.
pub fn snap_edge(distances: EdgeDistances, scale_factor: f64) -> Option<FloatEdge> {
    let scale = scale_factor.max(0.01);
    let snap = SNAP_DISTANCE * scale;
    let release = RELEASE_DISTANCE * scale;

    if distances.within_snap(FloatEdge::Top, snap, release) {
        return Some(FloatEdge::Top);
    }
    let left = distances.within_snap(FloatEdge::Left, snap, release);
    let right = distances.within_snap(FloatEdge::Right, snap, release);
    match (left, right) {
        (true, false) => Some(FloatEdge::Left),
        (false, true) => Some(FloatEdge::Right),
        (true, true) => Some(if distances.left <= distances.right {
            FloatEdge::Left
        } else {
            FloatEdge::Right
        }),
        (false, false) => None,
    }
}

/// Clamps a physical window position into one monitor's visible bounds.
///
/// Used by the non-Windows restore path and by the controller when an
/// un-docked side rail is expanded back into the configured layout; the
/// Windows adapter clamps against the native work area (taskbar-aware) instead.
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

    fn distances(top: f64, left: f64, right: f64) -> EdgeDistances {
        EdgeDistances { top, left, right }
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
            size(FloatPresentation::DockedTop, 2),
            (FLOAT_WIDTH, DOCKED_THICKNESS)
        );
        for presentation in [
            FloatPresentation::DockedLeft,
            FloatPresentation::DockedRight,
        ] {
            assert_eq!(
                size(presentation, 2),
                (DOCKED_THICKNESS, SIDE_DOCKED_DEFAULT_LENGTH),
                "a side rail is vertical"
            );
        }
    }

    #[test]
    fn docked_width_hugs_content_and_stays_in_range() {
        assert_eq!(docked_size(0.0), (DOCKED_MIN_LENGTH, DOCKED_THICKNESS));
        assert_eq!(
            docked_size(248.0),
            (248.0, DOCKED_THICKNESS),
            "a measured width is used as-is"
        );
        assert_eq!(
            docked_size(FLOAT_WIDTH + 80.0),
            (FLOAT_WIDTH, DOCKED_THICKNESS),
            "never wider than the Full/Compact window"
        );
        assert_eq!(
            docked_size(f64::NAN),
            (FLOAT_WIDTH, DOCKED_THICKNESS),
            "a bad report falls back to the full width"
        );
    }

    #[test]
    fn side_rail_length_hugs_content_and_stays_in_range() {
        assert_eq!(side_docked_size(0.0), (DOCKED_THICKNESS, DOCKED_MIN_LENGTH));
        assert_eq!(
            side_docked_size(264.0),
            (DOCKED_THICKNESS, 264.0),
            "a measured strip length becomes the rail height"
        );
        assert_eq!(
            side_docked_size(SIDE_DOCKED_MAX_LENGTH + 400.0),
            (DOCKED_THICKNESS, SIDE_DOCKED_MAX_LENGTH),
            "a long label cannot grow the rail past the Full window cap"
        );
        assert_eq!(
            side_docked_size(f64::NAN),
            (DOCKED_THICKNESS, SIDE_DOCKED_DEFAULT_LENGTH),
            "a bad report falls back to the default rail length"
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
        assert_eq!(corner_radius(FloatPresentation::Full, false), 16.0);
        assert_eq!(corner_radius(FloatPresentation::Compact, false), 16.0);
        for presentation in [
            FloatPresentation::DockedTop,
            FloatPresentation::DockedLeft,
            FloatPresentation::DockedRight,
        ] {
            assert_eq!(
                corner_radius(presentation, false),
                12.0,
                "a collapsed strip stays a capsule"
            );
            assert_eq!(
                corner_radius(presentation, true),
                16.0,
                "the hover card shares the Full/Compact radius"
            );
        }
    }

    #[test]
    fn hover_card_keeps_the_full_width_and_grows_past_the_strip() {
        let size = dock_hover_size();
        assert_eq!(size, (FLOAT_WIDTH, DOCK_HOVER_HEIGHT));
        assert!(
            size.1 > DOCKED_THICKNESS,
            "the card is taller than the strip"
        );
        assert!(
            DOCK_HOVER_HEIGHT < full_height(1),
            "the card must stay smaller than the Full layout"
        );
    }

    #[test]
    fn docked_position_anchors_each_edge_and_clamps_into_the_work_area() {
        let work = (0, 0, 1920, 1040);
        // The top strip grows downward from its own x; a strip near the right
        // edge is shifted left so the card fits, and collapses to the same x.
        assert_eq!(
            docked_position(FloatEdge::Top, 300, work, (392, 176)),
            (300, 0)
        );
        assert_eq!(
            docked_position(FloatEdge::Top, 1800, work, (392, 176)),
            (1528, 0)
        );
        assert_eq!(
            docked_position(FloatEdge::Top, 300, work, (392, 24)),
            (300, 0),
            "collapsing uses the stored strip anchor"
        );

        // Side rails grow away from the edge and keep their anchor on the
        // other axis, clamped to the work area.
        assert_eq!(
            docked_position(FloatEdge::Left, 300, work, (392, 176)),
            (0, 300)
        );
        assert_eq!(
            docked_position(FloatEdge::Right, 300, work, (392, 176)),
            (1528, 300)
        );
        assert_eq!(
            docked_position(FloatEdge::Right, 940, work, (392, 176)),
            (1528, 864),
            "a card opened near the bottom is pushed up"
        );
        assert_eq!(
            docked_position(FloatEdge::Left, -40, work, (24, 280)),
            (0, 0),
            "a rail dragged past the top is pushed back in"
        );
        assert_eq!(
            docked_position(FloatEdge::Left, 300, work, (4000, 2000)),
            (0, 0),
            "a card larger than the work area still starts at its corner"
        );
    }

    #[test]
    fn edge_distances_are_measured_from_the_work_area() {
        // A 392x280 window flush to the top-left of a 1920x1040 work area.
        let flush = EdgeDistances::new((0, 0), (392, 280), (0, 0, 1920, 1040));
        assert_eq!(
            flush,
            EdgeDistances {
                top: 0.0,
                left: 0.0,
                right: 1528.0
            }
        );

        // Flush to the right edge of a secondary monitor at x = -1920.
        let right = EdgeDistances::new((-392, 300), (392, 280), (-1920, 0, 0, 1040));
        assert_eq!(right.left, 1528.0);
        assert_eq!(right.right, 0.0);
        assert_eq!(right.top, 300.0);

        // Dragged past an edge is negative, not wrapped.
        let past = EdgeDistances::new((-30, -40), (392, 280), (0, 0, 1920, 1040));
        assert_eq!(past.left, -30.0);
        assert_eq!(past.top, -40.0);
    }

    #[test]
    fn top_snap_uses_hysteresis() {
        assert_eq!(
            plan_dock(None, false, distances(18.0, 500.0, 500.0), 1.0).edge,
            Some(FloatEdge::Top)
        );
        assert_eq!(
            plan_dock(None, false, distances(19.0, 500.0, 500.0), 1.0).edge,
            None
        );
        assert_eq!(
            plan_dock(
                Some(FloatEdge::Top),
                false,
                distances(24.0, 500.0, 500.0),
                1.0
            )
            .edge,
            Some(FloatEdge::Top)
        );
        let left = plan_dock(
            Some(FloatEdge::Top),
            false,
            distances(25.0, 500.0, 500.0),
            1.0,
        );
        assert_eq!(left.edge, None, "the top strip un-docks past the band");
        assert!(left.guarded, "an un-dock re-arms docking");
        assert_eq!(
            plan_dock(None, false, distances(-25.0, 500.0, 500.0), 1.0).edge,
            None,
            "a window dragged above the work area does not dock"
        );
        assert_eq!(
            plan_dock(
                Some(FloatEdge::Top),
                false,
                distances(-20.0, 500.0, 500.0),
                1.0
            )
            .edge,
            Some(FloatEdge::Top),
            "still inside the release band above the work area"
        );
    }

    #[test]
    fn docking_thresholds_scale_with_monitor_dpi() {
        assert_eq!(
            plan_dock(None, false, distances(27.0, 500.0, 500.0), 1.5).edge,
            Some(FloatEdge::Top)
        );
        assert_eq!(
            plan_dock(None, false, distances(28.0, 500.0, 500.0), 1.5).edge,
            None
        );
        assert_eq!(
            plan_dock(
                Some(FloatEdge::Top),
                false,
                distances(36.0, 500.0, 500.0),
                1.5
            )
            .edge,
            Some(FloatEdge::Top)
        );
        assert_eq!(
            plan_dock(
                Some(FloatEdge::Top),
                false,
                distances(37.0, 500.0, 500.0),
                1.5
            )
            .edge,
            None
        );
    }

    #[test]
    fn docking_tolerates_a_non_positive_scale_factor() {
        assert_eq!(
            plan_dock(None, false, distances(0.0, 500.0, 500.0), 0.0).edge,
            Some(FloatEdge::Top)
        );
        assert_eq!(
            plan_dock(None, false, distances(1.0, 500.0, 500.0), 0.0).edge,
            None
        );
    }

    #[test]
    fn side_edges_dock_to_the_vertical_rail() {
        let left = plan_dock(None, false, distances(400.0, 0.0, 800.0), 1.0);
        assert_eq!(left.edge, Some(FloatEdge::Left));
        assert!(!left.pending);

        let right = plan_dock(None, false, distances(400.0, 800.0, 0.0), 1.0);
        assert_eq!(right.edge, Some(FloatEdge::Right));
        assert!(!right.pending);

        assert_eq!(
            plan_dock(None, false, distances(400.0, 18.0, 800.0), 1.0).edge,
            Some(FloatEdge::Left)
        );
        assert_eq!(
            plan_dock(None, false, distances(400.0, 19.0, 800.0), 1.0).edge,
            None
        );
        assert_eq!(
            plan_dock(None, false, distances(400.0, -25.0, 800.0), 1.0).edge,
            None,
            "a window dragged past the side edge does not dock"
        );
    }

    #[test]
    fn top_keeps_priority_in_a_corner() {
        assert_eq!(
            plan_dock(None, false, distances(5.0, 0.0, 800.0), 1.0).edge,
            Some(FloatEdge::Top),
            "the corner belongs to the top strip"
        );
        assert_eq!(
            plan_dock(None, false, distances(19.0, 0.0, 800.0), 1.0).edge,
            Some(FloatEdge::Left),
            "past the top band the nearest side wins"
        );
    }

    #[test]
    fn side_rails_keep_the_strip_until_the_drag_settles() {
        let plan = plan_dock(
            Some(FloatEdge::Left),
            false,
            distances(400.0, 30.0, 800.0),
            1.0,
        );
        assert_eq!(plan.edge, Some(FloatEdge::Left));
        assert!(plan.pending, "a side rail expands after the drag settles");
        assert!(!plan.guarded);

        let back = plan_dock(
            Some(FloatEdge::Left),
            false,
            distances(400.0, 24.0, 800.0),
            1.0,
        );
        assert_eq!(back.edge, Some(FloatEdge::Left));
        assert!(
            !back.pending,
            "re-entering the band cancels the pending undock"
        );
    }

    #[test]
    fn guard_blocks_docking_until_every_band_is_left() {
        // Freshly un-docked at the top while still flush left: the left band
        // would otherwise re-dock the window to a side rail immediately.
        let blocked = plan_dock(None, true, distances(400.0, 0.0, 800.0), 1.0);
        assert_eq!(blocked.edge, None);
        assert!(blocked.guarded);

        let clear = plan_dock(None, true, distances(400.0, 60.0, 60.0), 1.0);
        assert_eq!(clear.edge, None);
        assert!(!clear.guarded, "leaving every band re-arms docking");

        // Once re-armed the next approach docks normally.
        assert_eq!(
            plan_dock(None, false, distances(400.0, 0.0, 800.0), 1.0).edge,
            Some(FloatEdge::Left)
        );
    }

    #[test]
    fn snap_edge_requires_the_top_or_a_side_inside_its_band() {
        assert_eq!(snap_edge(distances(400.0, 400.0, 400.0), 1.0), None);
        assert_eq!(
            snap_edge(distances(10.0, 5.0, 5.0), 1.0),
            Some(FloatEdge::Top)
        );
        assert_eq!(
            snap_edge(distances(400.0, 5.0, 5.0), 1.0),
            Some(FloatEdge::Left),
            "equal side distances prefer the left edge"
        );
        assert_eq!(
            snap_edge(distances(400.0, 12.0, 5.0), 1.0),
            Some(FloatEdge::Right)
        );
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
