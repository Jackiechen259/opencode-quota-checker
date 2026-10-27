//! Regression tests for the floating-window state machine.
//!
//! These pin the contract that fixes the "Full UI rendered inside a Compact
//! native window" bug: `AppConfig.float_mode` is the single persisted layout,
//! `FloatRuntime` only carries the transient dock flag, and the DTO's
//! `presentation_mode` is always derived from those two. Docked must never be
//! persisted, and the native size helpers must stay deterministic.

use opencode_core::{QuotaService, UsageReport, WindowReport};
use opencode_quota_checker_lib::config::{AppConfig, FloatLayout};
use opencode_quota_checker_lib::float::layout::{
    full_height, quota_count, size, COMPACT_HEIGHT, DOCKED_HEIGHT, FLOAT_WIDTH, FULL_MAX_HEIGHT,
    FULL_MIN_HEIGHT, SKELETON_CARD_COUNT,
};
use opencode_quota_checker_lib::float::state::{FloatPresentation, FloatRuntime};
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

fn docked(state: &AppState) -> bool {
    state.floating.lock().expect("float mutex").is_docked()
}

#[test]
fn initial_snapshot_is_full_and_undocked() {
    let state = state();
    let dto = state.float_dto();
    assert!(!dto.open);
    assert_eq!(dto.configured_mode, FloatLayout::Full);
    assert_eq!(dto.presentation_mode, FloatPresentation::Full);
    assert!(!dto.top_docked);
}

#[test]
fn layout_transitions_full_to_compact_and_back() {
    let state = state();
    state.apply_float_layout(FloatLayout::Compact);
    assert_eq!(configured(&state), FloatLayout::Compact);
    assert!(!docked(&state));
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
    state.floating.lock().expect("float mutex").set_docked(true);

    assert_eq!(
        configured(&state),
        FloatLayout::Compact,
        "docking must not persist"
    );
    assert!(docked(&state));
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatLayout::Compact);
    assert_eq!(dto.presentation_mode, FloatPresentation::Docked);
    assert!(dto.top_docked);

    // Choosing a layout leaves the transient docked presentation.
    state.apply_float_layout(FloatLayout::Full);
    assert!(!docked(&state));
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatLayout::Full);
    assert_eq!(dto.presentation_mode, FloatPresentation::Full);
}

#[test]
fn both_configured_layouts_can_dock_and_undock() {
    let state = state();
    for layout in [FloatLayout::Full, FloatLayout::Compact] {
        state.apply_float_layout(layout);
        state.floating.lock().expect("float mutex").set_docked(true);
        assert_eq!(
            state.float_dto().presentation_mode,
            FloatPresentation::Docked
        );

        state.floating.lock().expect("float mutex").clear_docked();
        assert_eq!(state.float_dto().presentation_mode, layout.into());
    }
}

#[test]
fn docked_is_never_written_to_the_persisted_config() {
    let state = state();
    state.apply_float_layout(FloatLayout::Full);
    state.floating.lock().expect("float mutex").set_docked(true);
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
        size(FloatPresentation::Docked, 2),
        (FLOAT_WIDTH, DOCKED_HEIGHT)
    );
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
    assert!(!dto.top_docked);

    state.floating.lock().expect("float mutex").set_docked(true);
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, configured(&state));
    assert_eq!(dto.presentation_mode, FloatPresentation::Docked);
    assert!(dto.top_docked);
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
