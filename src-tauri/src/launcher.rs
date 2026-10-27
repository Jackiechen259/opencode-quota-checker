//! Opening URLs in the system default browser.
//!
//! Ported from the archived Iced client (`opencode_login.rs` /
//! `update/installer.rs::open_url`). The app no longer captures a browser
//! session cookie: the user creates a scoped API key in the OpenCode Console
//! and pastes it into the credentials form, so this module only opens the
//! Console.

/// OpenCode Console: where API keys are created and usage lives.
pub const CONSOLE_URL: &str = "https://opencode.ai/console";

/// Opens `url` in the system default browser.
pub fn open_url(url: &str) -> Result<(), String> {
    let result = spawn_browser(url);
    result.map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn spawn_browser(url: &str) -> Result<(), std::io::Error> {
    // `cmd /C start "" <url>` launches the default browser without blocking.
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "macos")]
fn spawn_browser(url: &str) -> Result<(), std::io::Error> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_browser(url: &str) -> Result<(), std::io::Error> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
}
