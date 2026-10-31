//! Regression tests for the floating-window state machine.
//!
//! These pin the contract that fixes the "Full UI rendered inside a Compact
//! native window" bug: `AppConfig.float_mode` is the single persisted layout,
//! `FloatRuntime` only carries the transient dock edge, and the DTO's
//! `presentation_mode` is always derived from those two. Docked presentations
//! must never be persisted, and the native size helpers must stay
//! deterministic.

use opencode_core::{QuotaService, UsageReport, WindowReport};
use opencode_quota_checker_lib::config::{AppConfig, FloatLayout};
use opencode_quota_checker_lib::float::layout::{
    full_height, quota_count, size, COMPACT_HEIGHT, DOCKED_THICKNESS, FLOAT_WIDTH, FULL_MAX_HEIGHT,
    FULL_MIN_HEIGHT, SIDE_DOCKED_DEFAULT_LENGTH, SKELETON_CARD_COUNT,
};
use opencode_quota_checker_lib::float::state::{
    DockPlan, FloatEdge, FloatPresentation, FloatRuntime,
};
use opencode_quota_checker_lib::state::AppState;
use std::sync::Arc;
use tokio::sync::watch;

fn state() -> Arc<AppState> {
    let (tx, _rx) = watch::channel(None);
    Arc::new(AppState::new(
        QuotaService::new().expect("quota service"),
        tx,
    ))
}

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

fn report(count: usize) -> UsageReport {
    UsageReport {
        plan_type: "pro".to_owned(),
        fetched_at: 0,
        windows: (0..count)
            .map(|index| window(&format!("w{index}")))
            .collect(),
    }
}

fn configured(state: &AppState) -> FloatLayout {
    state.config.lock().expect("config mutex").float_mode
}

fn dock_edge(state: &AppState) -> Option<FloatEdge> {
    state.floating.lock().expect("float mutex").dock_edge()
}

#[test]
fn initial_snapshot_is_full_and_undocked() {
    let state = state();
    let dto = state.float_dto();
    assert!(!dto.open);
    assert_eq!(dto.configured_mode, FloatLayout::Full);
    assert_eq!(dto.presentation_mode, FloatPresentation::Full);
    assert!(
        !dto.dock_hover,
        "nothing can be hovering an unopened window"
    );
    assert_eq!(dock_edge(&state), None);
}

#[test]
fn dock_hover_expands_a_docked_strip_only() {
    let state = state();
    {
        let mut runtime = state.floating.lock().expect("float mutex");
        // Undocked: the flag is recorded but never reaches the DTO.
        runtime.set_dock_hover(true, Some(120));
    }
    assert!(!state.float_dto().dock_hover);

    {
        let mut runtime = state.floating.lock().expect("float mutex");
        runtime.apply_dock_plan(DockPlan::docked(FloatEdge::Left));
        runtime.set_dock_hover(true, Some(300));
    }
    let dto = state.float_dto();
    assert!(dto.dock_hover);
    assert_eq!(
        dto.presentation_mode,
        FloatPresentation::DockedLeft,
        "the hover card is a presentation flag, not a mode"
    );
    assert_eq!(
        configured(&state),
        FloatLayout::Full,
        "hovering must never persist anything"
    );

    // Un-docking drops the card with the docked presentation.
    state.floating.lock().expect("float mutex").clear_docked();
    let dto = state.float_dto();
    assert!(!dto.dock_hover);
    assert_eq!(dto.presentation_mode, FloatPresentation::Full);
}

#[test]
fn layout_transitions_full_to_compact_and_back() {
    let state = state();
    state.apply_float_layout(FloatLayout::Compact);
    assert_eq!(configured(&state), FloatLayout::Compact);
    assert_eq!(dock_edge(&state), None);
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatLayout::Compact);
    assert_eq!(dto.presentation_mode, FloatPresentation::Compact);

    state.apply_float_layout(FloatLayout::Full);
    assert_eq!(configured(&state), FloatLayout::Full);
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatLayout::Full);
    assert_eq!(dto.presentation_mode, FloatPresentation::Full);
}

#[test]
fn docking_is_transient_and_layout_change_undocks() {
    let state = state();
    state.apply_float_layout(FloatLayout::Compact);
    state
        .floating
        .lock()
        .expect("float mutex")
        .apply_dock_plan(DockPlan::docked(FloatEdge::Top));

    assert_eq!(
        configured(&state),
        FloatLayout::Compact,
        "docking must not persist"
    );
    assert_eq!(dock_edge(&state), Some(FloatEdge::Top));
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatLayout::Compact);
    assert_eq!(dto.presentation_mode, FloatPresentation::DockedTop);

    // Choosing a layout leaves the transient docked presentation.
    state.apply_float_layout(FloatLayout::Full);
    assert_eq!(dock_edge(&state), None);
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatLayout::Full);
    assert_eq!(dto.presentation_mode, FloatPresentation::Full);
}

#[test]
fn every_edge_has_its_own_docked_presentation() {
    let state = state();
    for (edge, presentation) in [
        (FloatEdge::Top, FloatPresentation::DockedTop),
        (FloatEdge::Left, FloatPresentation::DockedLeft),
        (FloatEdge::Right, FloatPresentation::DockedRight),
    ] {
        state.apply_float_layout(FloatLayout::Compact);
        state
            .floating
            .lock()
            .expect("float mutex")
            .apply_dock_plan(DockPlan::docked(edge));
        let dto = state.float_dto();
        assert_eq!(
            dto.configured_mode,
            FloatLayout::Compact,
            "the configured layout survives every dock edge"
        );
        assert_eq!(dto.presentation_mode, presentation);
        assert_eq!(
            dto.presentation_mode.docked_edge(),
            Some(edge),
            "the presentation carries its own edge"
        );

        state.floating.lock().expect("float mutex").clear_docked();
        assert_eq!(
            state.float_dto().presentation_mode,
            FloatPresentation::Compact
        );
    }
}

#[test]
fn a_pending_side_rail_stays_docked_until_it_expands() {
    let state = state();
    state
        .floating
        .lock()
        .expect("float mutex")
        .apply_dock_plan(DockPlan::pending_undock(FloatEdge::Right));

    let dto = state.float_dto();
    assert_eq!(
        dto.presentation_mode,
        FloatPresentation::DockedRight,
        "the rail presentation must follow the native rail size while pending"
    );
    {
        let mut runtime = state.floating.lock().expect("float mutex");
        assert!(runtime.is_undock_pending());
        assert!(!runtime.is_dock_guarded());
        assert!(runtime.apply_dock_plan(DockPlan::undocked(true)));
    }
    assert_eq!(state.float_dto().presentation_mode, FloatPresentation::Full);
    assert!(state
        .floating
        .lock()
        .expect("float mutex")
        .is_dock_guarded());
}

#[test]
fn docked_is_never_written_to_the_persisted_config() {
    let state = state();
    state.apply_float_layout(FloatLayout::Full);
    state
        .floating
        .lock()
        .expect("float mutex")
        .apply_dock_plan(DockPlan::docked(FloatEdge::Left));
    assert_eq!(configured(&state), FloatLayout::Full);

    // Legacy files may contain `"docked"`; it loads as the persisted Compact
    // layout instead of reintroducing a transient variant into the schema.
    let config: AppConfig =
        serde_json::from_str(r#"{"float_mode":"docked"}"#).expect("legacy config loads");
    assert_eq!(config.float_mode, FloatLayout::Compact);
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
fn size_for_full_mode_uses_the_quota_count() {
    let skeleton = size(FloatPresentation::Full, SKELETON_CARD_COUNT);
    assert_eq!(skeleton.0, FLOAT_WIDTH);
    assert!((FULL_MIN_HEIGHT..=FULL_MAX_HEIGHT).contains(&skeleton.1));

    let grown = size(FloatPresentation::Full, 4);
    assert!(grown.1 > skeleton.1);
    assert_eq!(
        size(FloatPresentation::Full, 5).1,
        FULL_MAX_HEIGHT,
        "five cards already hit the cap"
    );
}

#[test]
fn size_for_fixed_modes_is_constant() {
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
            "side rails are vertical"
        );
    }
}

#[test]
fn quota_count_uses_a_skeleton_and_at_least_one_card() {
    assert_eq!(quota_count(None), SKELETON_CARD_COUNT);
    let empty = report(0);
    assert_eq!(quota_count(Some(&empty)), 1);
    let four = report(4);
    assert_eq!(quota_count(Some(&four)), 4);
}

#[test]
fn float_dto_never_leaks_a_stale_mode() {
    let state = state();
    state.apply_float_layout(FloatLayout::Compact);
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, configured(&state));
    assert_eq!(dto.presentation_mode, FloatPresentation::Compact);
    assert_eq!(dock_edge(&state), None);

    state
        .floating
        .lock()
        .expect("float mutex")
        .apply_dock_plan(DockPlan::docked(FloatEdge::Right));
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, configured(&state));
    assert_eq!(dto.presentation_mode, FloatPresentation::DockedRight);
}

#[test]
fn runtime_move_generation_is_monotonic() {
    let mut runtime = FloatRuntime::default();
    let first = runtime.begin_move();
    let second = runtime.begin_move();
    assert!(second > first);
    assert!(!runtime.settle_move(first));
    assert!(runtime.settle_move(second));
}
