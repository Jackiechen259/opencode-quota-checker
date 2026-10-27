//! Floating-window subsystem.
//!
//! The module is split into three layers:
//!
//! * [`layout`] — pure geometry (sizes, docking hysteresis, clamp, radius);
//! * [`state`] — the transient runtime state and the IPC snapshot;
//! * [`controller`] — the only code allowed to create/destroy the native
//!   window, emit `float://state`, or persist the floating-window fields.
//!
//! `Docked` is a transient presentation: it never enters `AppConfig`. The
//! persisted layout lives in `AppConfig::float_mode`; the source of truth for
//! this subsystem is `FloatLayout` + [`state::FloatRuntime`]'s dock flag.

pub mod controller;
pub mod layout;
pub mod state;

#[cfg(target_os = "windows")]
pub(crate) mod win;

pub use state::{FloatPresentation, FloatRuntime, FloatStateDto};
