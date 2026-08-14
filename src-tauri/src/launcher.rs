//! Opening URLs in the system default browser.
//!
//! Ported from the archived Iced client (`opencode_login.rs` /
//! `update/installer.rs::open_url`). opencode.ai authenticates through an OAuth
//! flow hosted at `auth.opencode.ai` whose callback is pinned to
//! `opencode.ai/auth/callback`, so the session cookie is only ever set on the
//! opencode.ai origin. The login therefore opens in the system browser and the
//! manual paste form captures the Workspace ID and the `auth` cookie.

/// Entry point of the OpenCode OAuth login flow.
pub const LOGIN_URL: &str = "https://opencode.ai/auth";

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
