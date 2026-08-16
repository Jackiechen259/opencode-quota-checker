//! Regression tests for the floating-window state machine.
//!
//! These pin the contract that fixes the "Full UI rendered inside a Compact
//! native window" bug: `AppConfig.float_mode` is the single persisted source
//! of the configured mode, `top_docked` is the only transient flag, and
//! `presentation_mode` (what the UI renders and what the native window is
//! sized to) is always derived from those two. The DTO must never carry a
//! stale duplicate of the mode, and Docked must never be persisted.

use opencode_core::{QuotaService, UsageReport, WindowReport};
use opencode_quota_checker_lib::config::{AppConfig, FloatMode};
use opencode_quota_checker_lib::state::AppState;
use opencode_quota_checker_lib::window::float_window::{
    full_height, size_for, COMPACT_HEIGHT, DOCKED_HEIGHT, FLOAT_WIDTH, FULL_MAX_HEIGHT,
    FULL_MIN_HEIGHT,
};
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
    }
}

fn configured(state: &AppState) -> FloatMode {
    state.config.lock().expect("config mutex").float_mode
}

fn top_docked(state: &AppState) -> bool {
    state.floating.lock().expect("float mutex").top_docked
}

#[test]
fn mode_transitions_full_to_compact() {
    let state = state();
    state.apply_float_mode(FloatMode::Compact);
    assert_eq!(configured(&state), FloatMode::Compact);
    assert!(!top_docked(&state));
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatMode::Compact);
    assert_eq!(dto.presentation_mode, FloatMode::Compact);
}

#[test]
fn mode_transitions_compact_to_full() {
    let state = state();
    state.apply_float_mode(FloatMode::Compact);
    state.apply_float_mode(FloatMode::Full);
    assert_eq!(configured(&state), FloatMode::Full);
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatMode::Full);
    assert_eq!(dto.presentation_mode, FloatMode::Full);
}

#[test]
fn mode_transitions_full_docked_full() {
    let state = state();
    state.apply_float_mode(FloatMode::Docked);
    assert_eq!(
        configured(&state),
        FloatMode::Full,
        "docking must not persist"
    );
    assert!(top_docked(&state));
    assert_eq!(state.float_dto().presentation_mode, FloatMode::Docked);

    // Undock (via the Expand action) restores the previous configured mode.
    state.apply_float_mode(FloatMode::Full);
    assert!(!top_docked(&state));
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatMode::Full);
    assert_eq!(dto.presentation_mode, FloatMode::Full);
}

#[test]
fn mode_transitions_compact_docked_compact() {
    let state = state();
    state.apply_float_mode(FloatMode::Compact);
    state.apply_float_mode(FloatMode::Docked);
    assert_eq!(
        configured(&state),
        FloatMode::Compact,
        "docking must not persist"
    );
    assert!(top_docked(&state));
    assert_eq!(state.float_dto().presentation_mode, FloatMode::Docked);

    state.apply_float_mode(FloatMode::Compact);
    assert!(!top_docked(&state));
    let dto = state.float_dto();
    assert_eq!(dto.configured_mode, FloatMode::Compact);
    assert_eq!(dto.presentation_mode, FloatMode::Compact);
}

#[test]
fn effective_mode_derives_from_configured_and_dock_flag() {
    let state = state();

    state.apply_float_mode(FloatMode::Full);
    assert_eq!(state.float_dto().presentation_mode, FloatMode::Full);

    state.apply_float_mode(FloatMode::Compact);
    assert_eq!(state.float_dto().presentation_mode, FloatMode::Compact);

    state.apply_float_mode(FloatMode::Docked);
    assert_eq!(state.float_dto().presentation_mode, FloatMode::Docked);

    // Direct dock flag flip (native drag path) with each configured mode.
    for mode in [FloatMode::Full, FloatMode::Compact] {
        state.apply_float_mode(mode);
        state.floating.lock().expect("float mutex").top_docked = true;
        assert_eq!(state.float_dto().presentation_mode, FloatMode::Docked);
        state.floating.lock().expect("float mutex").top_docked = false;
        assert_eq!(state.float_dto().presentation_mode, mode);
    }
}

#[test]
fn docked_is_never_written_to_the_persisted_config() {
    let state = state();
    state.apply_float_mode(FloatMode::Docked);
    assert_ne!(configured(&state), FloatMode::Docked);

    // A config containing Docked is normalized on load/save validation.
    let config = AppConfig {
        float_mode: FloatMode::Docked,
        ..AppConfig::default()
    };
    assert_eq!(
        config.validate().expect("valid").float_mode,
        FloatMode::Compact
    );
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
    assert_eq!(
        full_height(5),
        FULL_MAX_HEIGHT,
        "five cards already hit the cap"
    );
    assert_eq!(
        full_height(100),
        FULL_MAX_HEIGHT,
        "many cards clamp to the cap"
    );
}

#[test]
fn size_for_full_mode_uses_usage_state() {
    let state = state();
    // No report yet: skeleton height for two cards.
    let (width, height) = size_for(FloatMode::Full, &state);
    assert_eq!(width, FLOAT_WIDTH);
    assert!((FULL_MIN_HEIGHT..=FULL_MAX_HEIGHT).contains(&height));
    let skeleton_height = height;

    // A report with several windows grows the window.
    {
        let mut usage = state.usage.lock().expect("usage mutex");
        usage.report = Some(UsageReport {
            plan_type: "pro".to_owned(),
            fetched_at: 0,
            windows: vec![window("a"), window("b"), window("c"), window("d")],
        });
    }
    let (_, grown) = size_for(FloatMode::Full, &state);
    assert!(grown > skeleton_height);
}

#[test]
fn size_for_fixed_modes_is_constant() {
    let state = state();
    assert_eq!(
        size_for(FloatMode::Compact, &state),
        (FLOAT_WIDTH, COMPACT_HEIGHT)
    );
    assert_eq!(
        size_for(FloatMode::Docked, &state),
        (FLOAT_WIDTH, DOCKED_HEIGHT)
    );
}

#[test]
fn float_dto_never_leaks_a_stale_mode() {
    let state = state();
    state.apply_float_mode(FloatMode::Compact);
    let dto = state.float_dto();
    // The DTO fields must agree with the canonical state at emission time;
    // there is no second `mode` copy left to go stale.
    assert_eq!(dto.configured_mode, configured(&state));
    assert_eq!(dto.presentation_mode, configured(&state));
    assert!(!dto.top_docked);
}
