//! Floating-window runtime state and the IPC snapshot.
//!
//! The model has exactly two inputs:
//!
//! * `AppConfig::float_mode` — the persisted layout (Full or Compact);
//! * [`FloatRuntime::dock_edge`] — the transient edge the window is snapped
//!   to: `Top` for the one-line strip, `Left`/`Right` for the vertical rail.
//!
//! Everything the UI and the native window consume is derived from those two
//! values by [`FloatRuntime::presentation`]. Keeping the derivation here (and
//! nowhere else) is what makes the DTO, the native size and the rendered view
//! unable to disagree.
//!
//! Side rails carry two extra transient flags that only matter while a drag is
//! in flight:
//!
//! * [`FloatRuntime::is_undock_pending`] — the rail has been dragged beyond its
//!   release band but keeps the rail presentation until the drag settles. The
//!   native move loop owns the window position while the mouse button is down,
//!   so resizing a right-edge rail to the wide layout mid-drag would leave most
//!   of the card off-screen.
//! * [`FloatRuntime::is_dock_guarded`] — set when a window un-docks, cleared
//!   once it leaves every snap band. It stops the freshly un-docked window from
//!   docking again (or snapping to a side it merely happened to be touching)
//!   on the next move event.
//!
//! A docked strip also carries the transient hover-card flag
//! ([`FloatRuntime::is_dock_hover`]). It is a presentation flag of the docked
//! window — it never changes the presentation, is never persisted and is
//! cleared whenever the window leaves (or starts to leave) its docked state.

use crate::config::FloatLayout;
use serde::{Deserialize, Serialize};

/// Screen edge a floating window can be snapped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatEdge {
    /// Single-line horizontal strip along the monitor work-area top.
    Top,
    /// Vertical rail against the work-area left edge.
    Left,
    /// Vertical rail against the work-area right edge.
    Right,
}

impl FloatEdge {
    /// Whether this edge renders as a vertical rail rather than a top strip.
    pub fn is_side(self) -> bool {
        !matches!(self, Self::Top)
    }
}

/// What the floating window must render and how it must be sized right now.
///
/// The docked presentations are not persisted and must never appear in
/// [`FloatLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FloatPresentation {
    /// All quota windows and controls.
    #[default]
    Full,
    /// Highest-risk quota summary.
    Compact,
    /// Single-line horizontal strip snapped to the monitor top.
    DockedTop,
    /// Vertical rail snapped to the left work-area edge.
    DockedLeft,
    /// Vertical rail snapped to the right work-area edge.
    DockedRight,
}

impl FloatPresentation {
    /// The edge this presentation is docked to, if any.
    pub fn docked_edge(self) -> Option<FloatEdge> {
        match self {
            Self::Full | Self::Compact => None,
            Self::DockedTop => Some(FloatEdge::Top),
            Self::DockedLeft => Some(FloatEdge::Left),
            Self::DockedRight => Some(FloatEdge::Right),
        }
    }
}

impl From<FloatLayout> for FloatPresentation {
    fn from(layout: FloatLayout) -> Self {
        match layout {
            FloatLayout::Full => Self::Full,
            FloatLayout::Compact => Self::Compact,
        }
    }
}

impl From<FloatEdge> for FloatPresentation {
    fn from(edge: FloatEdge) -> Self {
        match edge {
            FloatEdge::Top => Self::DockedTop,
            FloatEdge::Left => Self::DockedLeft,
            FloatEdge::Right => Self::DockedRight,
        }
    }
}

/// Result of one docking decision, applied atomically to [`FloatRuntime`].
///
/// The plan is produced by `float::layout::plan_dock` from the window's
/// distances to the work-area edges; the controller applies it in one step so
/// the edge, the pending flag and the guard can never disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockPlan {
    /// Edge the window presents as docked to; `None` means Full/Compact.
    pub edge: Option<FloatEdge>,
    /// A side rail is out of its release band but keeps the rail until the
    /// drag settles.
    pub pending: bool,
    /// Docking stays blocked until the window leaves every snap band.
    pub guarded: bool,
}

impl DockPlan {
    /// The window stays docked to `edge`.
    pub fn docked(edge: FloatEdge) -> Self {
        Self {
            edge: Some(edge),
            pending: false,
            guarded: false,
        }
    }

    /// The side rail has left its release band; expand after the drag settles.
    pub fn pending_undock(edge: FloatEdge) -> Self {
        Self {
            edge: Some(edge),
            pending: true,
            guarded: false,
        }
    }

    /// The window is (or stays) un-docked.
    pub fn undocked(guarded: bool) -> Self {
        Self {
            edge: None,
            pending: false,
            guarded,
        }
    }
}

/// Snapshot of the floating window crossing the IPC boundary.
///
/// `configuredMode` is always the persisted layout; `presentationMode` is the
/// derived presentation (one of `full`, `compact`, `docked_top`,
/// `docked_left`, `docked_right`). The frontend renders exclusively from
/// `presentationMode`; a docked presentation already carries its edge, so no
/// redundant `topDocked` flag crosses the boundary.
#[derive(Debug, Clone, Copy, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FloatStateDto {
    pub open: bool,
    pub configured_mode: FloatLayout,
    pub presentation_mode: FloatPresentation,
    /// The docked strip is expanded into its hover card.
    ///
    /// Always `false` while undocked: the hover card is a transient
    /// presentation of the docked strip, not a fourth layout.
    pub dock_hover: bool,
}

/// Transient floating-window state owned by the controller.
///
/// Fields are private so every transition goes through a method that preserves
/// the invariants:
///
/// * a docked presentation is only reachable while `dock_edge` is `Some`;
/// * `pending` is only meaningful while docked to a side edge;
/// * the position debounce accepts only the newest move generation;
/// * closing cancels any pending position write and any pending settle;
/// * `docked_length` only feeds the native size and stays out of the DTO.
#[derive(Debug, Default)]
pub struct FloatRuntime {
    open: bool,
    dock_edge: Option<FloatEdge>,
    undock_pending: bool,
    dock_guarded: bool,
    position_dirty: bool,
    position_generation: u64,
    /// Native strip length measured by the webview, if it has reported one.
    ///
    /// This is the Docked strip's width for the top edge and the rail's height
    /// for the side edges. It never leaves the process through the DTO and is
    /// never persisted; it only feeds the native size while docked.
    docked_length: Option<f64>,
    /// Whether the docked strip is expanded into its hover card.
    ///
    /// Hover is transient: it is cleared whenever the window leaves (or is
    /// about to leave) its docked presentation, and it never feeds
    /// `AppConfig`. Unlike the strip length it does cross the IPC boundary so
    /// the webview can render the card the native window has grown into.
    dock_hover: bool,
    /// Coordinate along the docked edge the strip had before expanding.
    ///
    /// Expanding can clamp the card into the work area, so collapsing restores
    /// this coordinate instead of the card's. Only meaningful while
    /// `dock_hover` is set.
    dock_hover_origin: Option<i32>,
}

impl FloatRuntime {
    /// Whether the floating window is currently considered open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Sets the open flag; returns `true` when it changed.
    pub fn set_open(&mut self, open: bool) -> bool {
        if self.open == open {
            return false;
        }
        self.open = open;
        true
    }

    /// The work-area edge the window is snapped to, if any.
    pub fn dock_edge(&self) -> Option<FloatEdge> {
        self.dock_edge
    }

    /// Whether the window is currently snapped to an edge.
    pub fn is_docked(&self) -> bool {
        self.dock_edge.is_some()
    }

    /// Whether a side rail waits for the drag to settle before expanding.
    pub fn is_undock_pending(&self) -> bool {
        self.undock_pending
    }

    /// Whether new docking is blocked until every snap band is left.
    pub fn is_dock_guarded(&self) -> bool {
        self.dock_guarded
    }

    /// Applies a resolved docking decision; returns `true` when the
    /// presentation changed.
    ///
    /// This is the only writer of the edge/pending/guard trio, so a plan can
    /// never be half-applied. Leaving the docked presentation (or starting to
    /// leave it: `pending`) also clears the hover expansion, so a card cannot
    /// survive an un-dock and land in the middle of the screen.
    pub fn apply_dock_plan(&mut self, plan: DockPlan) -> bool {
        let presentation_changed = self.dock_edge != plan.edge;
        self.dock_edge = plan.edge;
        self.undock_pending = plan.pending;
        self.dock_guarded = plan.guarded;
        if presentation_changed || plan.pending {
            self.clear_dock_hover();
        }
        presentation_changed
    }

    /// Updates the re-dock guard without touching the docking decision.
    ///
    /// Used by the controller after it has resized and repositioned an
    /// un-docking window: whether the result sits inside a snap band can only
    /// be measured once the native geometry is final.
    pub fn set_dock_guarded(&mut self, guarded: bool) -> bool {
        if self.dock_guarded == guarded {
            return false;
        }
        self.dock_guarded = guarded;
        true
    }

    /// Leaves the docked presentation (and clears every docking flag).
    ///
    /// Used when the configured layout changes: choosing a layout always
    /// leaves the transient presentation and re-arms docking.
    pub fn clear_docked(&mut self) -> bool {
        self.apply_dock_plan(DockPlan::undocked(false))
    }

    /// Cancels a pending side un-dock and re-arms docking.
    ///
    /// Used when the window closes: the already-spawned settle task must not
    /// resize a window that is going away, and the next open starts from a
    /// clean docking state while preserving the docked edge.
    pub fn cancel_dock_settle(&mut self) {
        self.undock_pending = false;
        self.dock_guarded = false;
    }

    /// The last native strip length measured by the webview.
    pub fn docked_length(&self) -> Option<f64> {
        self.docked_length
    }

    /// Records a measured strip length; returns `true` when it changed.
    pub fn set_docked_length(&mut self, length: f64) -> bool {
        if self.docked_length == Some(length) {
            return false;
        }
        self.docked_length = Some(length);
        true
    }

    /// Whether the docked strip is expanded into its hover card.
    pub fn is_dock_hover(&self) -> bool {
        self.dock_hover
    }

    /// The along-edge coordinate the strip had before expanding.
    ///
    /// `None` while collapsed.
    pub fn dock_hover_origin(&self) -> Option<i32> {
        self.dock_hover_origin
    }

    /// Expands (`hover = true`) or collapses the docked hover card.
    ///
    /// `origin` is the collapsed strip's along-edge coordinate and is only
    /// stored while expanding. Returns `true` when the state changed.
    pub fn set_dock_hover(&mut self, hover: bool, origin: Option<i32>) -> bool {
        if self.dock_hover == hover {
            return false;
        }
        self.dock_hover = hover;
        self.dock_hover_origin = if hover { origin } else { None };
        true
    }

    /// Moves the stored strip anchor after the expanded card was dragged along
    /// its edge, so the collapse returns to the dragged position.
    ///
    /// Returns `true` when it changed; a no-op while collapsed.
    pub fn set_dock_hover_origin(&mut self, origin: i32) -> bool {
        if !self.dock_hover || self.dock_hover_origin == Some(origin) {
            return false;
        }
        self.dock_hover_origin = Some(origin);
        true
    }

    /// Clears the hover expansion without touching the docking decision.
    pub fn clear_dock_hover(&mut self) {
        self.dock_hover = false;
        self.dock_hover_origin = None;
    }

    /// The presentation implied by the persisted layout and the dock edge.
    pub fn presentation(&self, layout: FloatLayout) -> FloatPresentation {
        match self.dock_edge {
            Some(edge) => edge.into(),
            None => layout.into(),
        }
    }

    /// Builds the canonical IPC snapshot.
    pub fn snapshot(&self, layout: FloatLayout) -> FloatStateDto {
        FloatStateDto {
            open: self.open,
            configured_mode: layout,
            presentation_mode: self.presentation(layout),
            // Hover is only representable while docked; the guard keeps a
            // stray flag from ever reaching the frontend.
            dock_hover: self.dock_hover && self.dock_edge.is_some(),
        }
    }

    /// Marks a move as pending and returns its generation.
    ///
    /// Only the newest generation is allowed to write the position (or run the
    /// dock settle) after the debounce interval; older spawned tasks become
    /// no-ops.
    pub fn begin_move(&mut self) -> u64 {
        self.position_dirty = true;
        self.position_generation = self.position_generation.wrapping_add(1);
        self.position_generation
    }

    /// Whether `generation` is still the newest move.
    pub fn is_latest_move(&self, generation: u64) -> bool {
        self.position_generation == generation
    }

    /// Consumes the pending move if `generation` is still the newest one.
    pub fn settle_move(&mut self, generation: u64) -> bool {
        if !self.position_dirty || self.position_generation != generation {
            return false;
        }
        self.position_dirty = false;
        true
    }

    /// Cancels any pending position write and settle; returns whether one was
    /// pending.
    ///
    /// Used when the window closes: the controller persists the latest
    /// position immediately, and the already-spawned debounce/settle tasks must
    /// not write or resize afterwards.
    pub fn take_pending_position(&mut self) -> bool {
        let pending = self.position_dirty;
        self.position_dirty = false;
        self.position_generation = self.position_generation.wrapping_add(1);
        pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_is_derived_from_layout_and_dock_edge() {
        let mut runtime = FloatRuntime::default();
        assert_eq!(
            runtime.presentation(FloatLayout::Full),
            FloatPresentation::Full
        );
        assert_eq!(
            runtime.presentation(FloatLayout::Compact),
            FloatPresentation::Compact
        );

        for (edge, expected) in [
            (FloatEdge::Top, FloatPresentation::DockedTop),
            (FloatEdge::Left, FloatPresentation::DockedLeft),
            (FloatEdge::Right, FloatPresentation::DockedRight),
        ] {
            assert!(runtime.apply_dock_plan(DockPlan::docked(edge)));
            assert_eq!(runtime.presentation(FloatLayout::Full), expected);
            assert_eq!(
                runtime.presentation(FloatLayout::Compact),
                expected,
                "a docked edge wins over the persisted layout"
            );
        }

        assert!(runtime.clear_docked());
        assert_eq!(
            runtime.presentation(FloatLayout::Compact),
            FloatPresentation::Compact
        );
        assert!(!runtime.is_docked());
        assert!(!runtime.is_undock_pending());
        assert!(!runtime.is_dock_guarded());
    }

    #[test]
    fn docked_presentations_report_their_edge() {
        assert_eq!(
            FloatPresentation::Full.docked_edge(),
            None,
            "Full is not docked"
        );
        assert_eq!(FloatPresentation::Compact.docked_edge(), None);
        assert_eq!(
            FloatPresentation::DockedTop.docked_edge(),
            Some(FloatEdge::Top)
        );
        assert_eq!(
            FloatPresentation::DockedLeft.docked_edge(),
            Some(FloatEdge::Left)
        );
        assert_eq!(
            FloatPresentation::DockedRight.docked_edge(),
            Some(FloatEdge::Right)
        );
        assert!(FloatEdge::Left.is_side());
        assert!(FloatEdge::Right.is_side());
        assert!(!FloatEdge::Top.is_side());
    }

    #[test]
    fn snapshot_never_carries_a_stale_mode() {
        let mut runtime = FloatRuntime::default();
        let dto = runtime.snapshot(FloatLayout::Compact);
        assert!(!dto.open);
        assert_eq!(dto.configured_mode, FloatLayout::Compact);
        assert_eq!(dto.presentation_mode, FloatPresentation::Compact);

        runtime.set_open(true);
        runtime.apply_dock_plan(DockPlan::docked(FloatEdge::Right));
        let dto = runtime.snapshot(FloatLayout::Compact);
        assert!(dto.open);
        assert_eq!(dto.configured_mode, FloatLayout::Compact);
        assert_eq!(dto.presentation_mode, FloatPresentation::DockedRight);
    }

    #[test]
    fn apply_dock_plan_reports_only_presentation_changes() {
        let mut runtime = FloatRuntime::default();
        assert!(runtime.apply_dock_plan(DockPlan::docked(FloatEdge::Left)));
        assert!(
            !runtime.apply_dock_plan(DockPlan::pending_undock(FloatEdge::Left)),
            "pending is a size/timing flag, not a new presentation"
        );
        assert!(runtime.is_undock_pending());
        assert!(
            !runtime.apply_dock_plan(DockPlan::pending_undock(FloatEdge::Left)),
            "repeating the same pending plan changes nothing"
        );
        assert!(runtime.apply_dock_plan(DockPlan::undocked(true)));
        assert!(!runtime.is_docked());
        assert!(runtime.is_dock_guarded());
        assert!(!runtime.is_undock_pending());
    }

    #[test]
    fn set_open_reports_only_real_changes() {
        let mut runtime = FloatRuntime::default();
        assert!(runtime.set_open(true));
        assert!(!runtime.set_open(true));
        assert!(runtime.set_open(false));
    }

    #[test]
    fn guard_updates_are_idempotent() {
        let mut runtime = FloatRuntime::default();
        assert!(runtime.set_dock_guarded(true));
        assert!(!runtime.set_dock_guarded(true));
        assert!(runtime.set_dock_guarded(false));
        assert!(!runtime.is_dock_guarded());
    }

    #[test]
    fn docked_length_reports_only_real_changes_and_never_leaks_into_the_dto() {
        let mut runtime = FloatRuntime::default();
        assert_eq!(runtime.docked_length(), None);

        assert!(runtime.set_docked_length(264.0));
        assert_eq!(runtime.docked_length(), Some(264.0));
        assert!(!runtime.set_docked_length(264.0));
        assert!(runtime.set_docked_length(300.0));

        let dto = runtime.snapshot(FloatLayout::Full);
        assert_eq!(dto.presentation_mode, FloatPresentation::Full);
        assert_eq!(
            serde_json::to_value(dto).expect("dto serializes"),
            serde_json::json!({
                "open": false,
                "configuredMode": "full",
                "presentationMode": "full",
                "dockHover": false,
            }),
            "the measured length must not cross the IPC boundary"
        );
    }

    #[test]
    fn dock_hover_is_transient_and_never_survives_an_undock() {
        let mut runtime = FloatRuntime::default();
        assert!(!runtime.is_dock_hover());

        // Hover is meaningless while the window is not docked, but the runtime
        // still records it so the controller can restore the size.
        assert!(runtime.set_dock_hover(true, Some(120)));
        assert!(!runtime.snapshot(FloatLayout::Full).dock_hover);

        runtime.apply_dock_plan(DockPlan::docked(FloatEdge::Top));
        assert!(runtime.set_dock_hover(true, Some(300)));
        assert!(runtime.is_dock_hover());
        assert_eq!(runtime.dock_hover_origin(), Some(300));
        let dto = runtime.snapshot(FloatLayout::Full);
        assert!(dto.dock_hover, "the expanded card crosses the IPC boundary");
        assert_eq!(dto.presentation_mode, FloatPresentation::DockedTop);

        assert!(
            !runtime.set_dock_hover(true, Some(999)),
            "re-expanding reports no change"
        );
        assert_eq!(
            runtime.dock_hover_origin(),
            Some(300),
            "a repeated expand keeps the strip anchor"
        );
        assert!(runtime.set_dock_hover_origin(420));
        assert_eq!(runtime.dock_hover_origin(), Some(420));
        assert!(
            !runtime.set_dock_hover_origin(420),
            "an unchanged dragged anchor reports no change"
        );

        assert!(runtime.set_dock_hover(false, None));
        assert!(!runtime.is_dock_hover());
        assert_eq!(runtime.dock_hover_origin(), None);

        // Leaving the docked presentation drops the card with it.
        runtime.set_dock_hover(true, Some(300));
        assert!(runtime.apply_dock_plan(DockPlan::undocked(true)));
        assert!(!runtime.is_dock_hover());
        assert_eq!(runtime.dock_hover_origin(), None);
        assert!(!runtime.snapshot(FloatLayout::Full).dock_hover);

        // A side rail that is only pending its un-dock is already collapsing.
        runtime.apply_dock_plan(DockPlan::docked(FloatEdge::Right));
        runtime.set_dock_hover(true, Some(80));
        assert!(!runtime.apply_dock_plan(DockPlan::pending_undock(FloatEdge::Right)));
        assert!(
            !runtime.is_dock_hover(),
            "a pending side un-dock must not leave the card open"
        );
    }

    #[test]
    fn clearing_the_dock_plan_drops_the_hover_card() {
        let mut runtime = FloatRuntime::default();
        runtime.apply_dock_plan(DockPlan::docked(FloatEdge::Left));
        runtime.set_dock_hover(true, Some(64));

        assert!(runtime.clear_docked());
        assert!(!runtime.is_dock_hover());
        assert!(!runtime.snapshot(FloatLayout::Compact).dock_hover);
    }

    #[test]
    fn only_the_latest_move_generation_settles() {
        let mut runtime = FloatRuntime::default();
        let first = runtime.begin_move();
        let second = runtime.begin_move();

        assert!(!runtime.is_latest_move(first));
        assert!(runtime.is_latest_move(second));
        assert!(!runtime.settle_move(first));
        assert!(!runtime.settle_move(second.wrapping_sub(2)));
        assert!(runtime.settle_move(second));
        assert!(!runtime.settle_move(second), "settling twice is a no-op");
    }

    #[test]
    fn taking_a_pending_position_cancels_the_generation() {
        let mut runtime = FloatRuntime::default();
        let generation = runtime.begin_move();

        assert!(runtime.take_pending_position());
        assert!(
            !runtime.is_latest_move(generation),
            "the settle/debounce tasks must lose to the immediate close write"
        );
        assert!(
            !runtime.settle_move(generation),
            "the debounce task must lose to the immediate close write"
        );
        assert!(!runtime.take_pending_position());
    }
}
