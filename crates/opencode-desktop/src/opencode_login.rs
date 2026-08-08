//! OpenCode login page launcher.
//!
//! opencode.ai authenticates through an OAuth authorization-code flow hosted at
//! `auth.opencode.ai` whose callback is pinned to `opencode.ai/auth/callback`,
//! so the session cookie is only ever set on the opencode.ai origin. The
//! application cannot capture that cookie from an embedded webview reliably: the
//! UI stack here is Iced 0.14, which has no WebView widget and does not expose
//! window handles, and a `wry` + `tao` login window cannot run on a background
//! thread next to Iced's main-thread event loop (the WebView2 message pump
//! blocks the loop; macOS allows only one main-thread loop). The login therefore
//! opens in the system browser, and the manual paste form below captures the
//! Workspace ID and the `auth` cookie.

/// Entry point of the OAuth login flow.
pub const LOGIN_URL: &str = "https://opencode.ai/auth";

/// Opens [`LOGIN_URL`] in the system default browser.
pub fn open_login_page() -> Result<(), String> {
    let result = spawn_browser();
    result.map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn spawn_browser() -> Result<(), std::io::Error> {
    // `cmd /C start "" <url>` launches the default browser without blocking.
    std::process::Command::new("cmd")
        .args(["/C", "start", "", LOGIN_URL])
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "macos")]
fn spawn_browser() -> Result<(), std::io::Error> {
    std::process::Command::new("open")
        .arg(LOGIN_URL)
        .spawn()
        .map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_browser() -> Result<(), std::io::Error> {
    std::process::Command::new("xdg-open")
        .arg(LOGIN_URL)
        .spawn()
        .map(|_| ())
}
