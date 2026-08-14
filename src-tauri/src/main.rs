// Release builds on Windows are GUI applications: launching them must not
// open a console window. Debug builds keep the console so development logs
// from `tracing_subscriber` remain visible under `cargo run`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    opencode_quota_checker_lib::run();
}
