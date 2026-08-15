//! Integration-test target.
//!
//! Deliberately tiny: its purpose is to give cargo a declared `[[test]]`
//! target so `build.rs` can pass `/MANIFEST:EMBED` +
//! `/MANIFESTINPUT:common-controls-v6.manifest` to **every** test harness
//! binary of this crate (lib unit tests included). Without the manifest the
//! test binaries import comctl32 v6 entry points (via `muda`/tauri's dialog
//! code) that comctl32 5.82 does not export, and Windows refuses to start
//! them with STATUS_ENTRYPOINT_NOT_FOUND.

#[test]
fn manifest_linkage_is_wired() {
    // The build script embeds the Common-Controls v6 manifest into test
    // binaries; verify the crate metadata is present in this binary.
    assert!(!env!("CARGO_PKG_VERSION").is_empty());
}
