//! Floating-window subsystem.
//!
//! The module is split into three layers:
//!
//! * [`layout`] — pure geometry (sizes, edge distances, docking hysteresis,
//!   clamp, radius);
//! * [`state`] — the transient runtime state and the IPC snapshot;
//! * [`controller`] — the only code allowed to create/destroy the native
//!   window, emit `float://state`, or persist the floating-window fields.
//!
//! The docked presentations (`DockedTop`, `DockedLeft`, `DockedRight`) are
//! transient: they never enter `AppConfig`. The persisted layout lives in
//! `AppConfig::float_mode`; the source of truth for this subsystem is
//! `FloatLayout` + [`state::FloatRuntime`]'s dock edge.

pub mod controller;
pub mod layout;
pub mod state;

#[cfg(target_os = "windows")]
pub(crate) mod win;

pub use state::{DockPlan, FloatEdge, FloatPresentation, FloatRuntime, FloatStateDto};
