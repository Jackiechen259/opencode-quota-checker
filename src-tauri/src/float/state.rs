//! Floating-window runtime state and the IPC snapshot.
//!
//! The model has exactly two inputs:
//!
//! * `AppConfig::float_mode` — the persisted layout (Full or Compact);
//! * [`FloatRuntime::docked`] — the transient presentation entered by dragging
//!   the window to the monitor top.
//!
//! Everything the UI and the native window consume is derived from those two
//! values by [`FloatRuntime::presentation`]. Keeping the derivation here (and
//! nowhere else) is what makes the DTO, the native size and the rendered view
//! unable to disagree.

use crate::config::FloatLayout;
use serde::{Deserialize, Serialize};

/// What the floating window must render and how it must be sized right now.
///
/// `Docked` is not persisted and must never appear in [`FloatLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FloatPresentation {
    /// All quota windows and controls.
    #[default]
    Full,
    /// Highest-risk quota summary.
    Compact,
    /// Single-line strip snapped to the monitor top.
    Docked,
}

impl From<FloatLayout> for FloatPresentation {
    fn from(layout: FloatLayout) -> Self {
        match layout {
            FloatLayout::Full => Self::Full,
            FloatLayout::Compact => Self::Compact,
        }
    }
}

/// Snapshot of the floating window crossing the IPC boundary.
///
/// The JSON shape is intentionally unchanged from the previous DTO:
/// `open`, `configuredMode`, `presentationMode`, `topDocked`. The frontend
/// renders exclusively from `presentation_mode`; `configured_mode` is exposed
/// for diagnostics and for the mode-toggle direction.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FloatStateDto {
    pub open: bool,
    pub configured_mode: FloatLayout,
    pub presentation_mode: FloatPresentation,
    pub top_docked: bool,
}

/// Transient floating-window state owned by the controller.
///
/// Fields are private so every transition goes through a method that preserves
/// the invariants:
///
/// * `Docked` is only reachable while `docked == true`;
/// * the position debounce accepts only the newest move generation;
/// * closing cancels any pending position write;
/// * `docked_width` only feeds the native size and stays out of the DTO.
#[derive(Debug, Default)]
pub struct FloatRuntime {
    open: bool,
    docked: bool,
    position_dirty: bool,
    position_generation: u64,
    /// Native Docked width measured by the webview, if it has reported one.
    ///
    /// This never leaves the process through the DTO and is never persisted;
    /// it only feeds the native size while the window is Docked.
    docked_width: Option<f64>,
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

    /// Whether the window is currently snapped to the monitor top.
    pub fn is_docked(&self) -> bool {
        self.docked
    }

    /// Sets the dock flag; returns `true` when it changed.
    pub fn set_docked(&mut self, docked: bool) -> bool {
        if self.docked == docked {
            return false;
        }
        self.docked = docked;
        true
    }

    /// Leaves the docked presentation when the configured layout changes.
    pub fn clear_docked(&mut self) -> bool {
        self.set_docked(false)
    }

    /// The last native width measured by the webview for the Docked strip.
    pub fn docked_width(&self) -> Option<f64> {
        self.docked_width
    }

    /// Records a measured Docked width; returns `true` when it changed.
    pub fn set_docked_width(&mut self, width: f64) -> bool {
        if self.docked_width == Some(width) {
            return false;
        }
        self.docked_width = Some(width);
        true
    }

    /// The presentation mode implied by the persisted layout and dock flag.
    pub fn presentation(&self, layout: FloatLayout) -> FloatPresentation {
        if self.docked {
            FloatPresentation::Docked
        } else {
            layout.into()
        }
    }

    /// Builds the canonical IPC snapshot.
    pub fn snapshot(&self, layout: FloatLayout) -> FloatStateDto {
        FloatStateDto {
            open: self.open,
            configured_mode: layout,
            presentation_mode: self.presentation(layout),
            top_docked: self.docked,
        }
    }

    /// Marks a move as pending and returns its generation.
    ///
    /// Only the newest generation is allowed to write the position after the
    /// debounce interval; older spawned tasks become no-ops.
    pub fn begin_move(&mut self) -> u64 {
        self.position_dirty = true;
        self.position_generation = self.position_generation.wrapping_add(1);
        self.position_generation
    }

    /// Consumes the pending move if `generation` is still the newest one.
    pub fn settle_move(&mut self, generation: u64) -> bool {
        if !self.position_dirty || self.position_generation != generation {
            return false;
        }
        self.position_dirty = false;
        true
    }

    /// Cancels any pending position write; returns whether one was pending.
    ///
    /// Used when the window closes: the controller persists the latest
    /// position immediately, and the already-spawned debounce task must not
    /// write again afterwards.
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
    fn presentation_is_derived_from_layout_and_dock_flag() {
        let mut runtime = FloatRuntime::default();
        assert_eq!(
            runtime.presentation(FloatLayout::Full),
            FloatPresentation::Full
        );
        assert_eq!(
            runtime.presentation(FloatLayout::Compact),
            FloatPresentation::Compact
        );

        assert!(runtime.set_docked(true));
        assert_eq!(
            runtime.presentation(FloatLayout::Full),
            FloatPresentation::Docked
        );
        assert_eq!(
            runtime.presentation(FloatLayout::Compact),
            FloatPresentation::Docked
        );

        assert!(runtime.clear_docked());
        assert_eq!(
            runtime.presentation(FloatLayout::Compact),
            FloatPresentation::Compact
        );
    }

    #[test]
    fn snapshot_never_carries_a_stale_mode() {
        let mut runtime = FloatRuntime::default();
        let dto = runtime.snapshot(FloatLayout::Compact);
        assert!(!dto.open);
        assert_eq!(dto.configured_mode, FloatLayout::Compact);
        assert_eq!(dto.presentation_mode, FloatPresentation::Compact);
        assert!(!dto.top_docked);

        runtime.set_open(true);
        runtime.set_docked(true);
        let dto = runtime.snapshot(FloatLayout::Compact);
        assert!(dto.open);
        assert_eq!(dto.configured_mode, FloatLayout::Compact);
        assert_eq!(dto.presentation_mode, FloatPresentation::Docked);
        assert!(dto.top_docked);
    }

    #[test]
    fn set_flags_report_only_real_changes() {
        let mut runtime = FloatRuntime::default();
        assert!(runtime.set_open(true));
        assert!(!runtime.set_open(true));
        assert!(runtime.set_open(false));

        assert!(runtime.set_docked(true));
        assert!(!runtime.set_docked(true));
        assert!(runtime.clear_docked());
        assert!(!runtime.clear_docked());
    }

    #[test]
    fn docked_width_reports_only_real_changes_and_never_leaks_into_the_dto() {
        let mut runtime = FloatRuntime::default();
        assert_eq!(runtime.docked_width(), None);

        assert!(runtime.set_docked_width(264.0));
        assert_eq!(runtime.docked_width(), Some(264.0));
        assert!(!runtime.set_docked_width(264.0));
        assert!(runtime.set_docked_width(300.0));

        let dto = runtime.snapshot(FloatLayout::Full);
        assert_eq!(dto.presentation_mode, FloatPresentation::Full);
        assert!(!dto.top_docked);
    }

    #[test]
    fn only_the_latest_move_generation_settles() {
        let mut runtime = FloatRuntime::default();
        let first = runtime.begin_move();
        let second = runtime.begin_move();

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
            !runtime.settle_move(generation),
            "the debounce task must lose to the immediate close write"
        );
        assert!(!runtime.take_pending_position());
    }
}
