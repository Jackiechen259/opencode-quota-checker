//! Native Windows window-shape helpers shared by the main and floating
//! windows.
//!
//! The webview surface is opaque on Windows, so CSS alone cannot cut a
//! transparent rounded corner out of the native window: the square HWND
//! corner would still show outside the CSS radius (a white wedge on a light
//! surface, especially obvious under the red close hover). A window region is
//! what actually shapes the HWND.
//!
//! The region is expressed in window coordinates and does not follow a resize
//! or DPI change, so callers must reapply it on `Resized` /
//! `ScaleFactorChanged` (and drop it while maximized).

use raw_window_handle::HasWindowHandle;
use tauri::WebviewWindow;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateRoundRectRgn, DeleteObject, SetWindowRgn,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};

/// Returns the native `HWND` of a live window.
pub fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    let handle = window.window_handle().ok()?.as_raw();
    match handle {
        raw_window_handle::RawWindowHandle::Win32(handle) => {
            Some(HWND(handle.hwnd.get() as *mut _))
        }
        _ => None,
    }
}

/// Clips the undecorated window to a rounded rectangle.
///
/// `radius` is in logical pixels and is converted with the window's current
/// DPI so the native curve and a CSS `border-radius` of the same value stay
/// on the same arc.
pub fn round_corners(window: &WebviewWindow, radius: f32) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    let mut window_rect = RECT::default();
    // SAFETY: `hwnd` is a valid live window handle and the pointer refers to
    // an initialized stack value.
    if unsafe { GetWindowRect(hwnd, &mut window_rect) }.is_err() {
        return;
    }
    let mut client_rect = RECT::default();
    // SAFETY: same handle; `GetClientRect` writes a 0,0,w,h rectangle.
    if unsafe { GetClientRect(hwnd, &mut client_rect) }.is_err() {
        return;
    }
    let mut client_origin = POINT { x: 0, y: 0 };
    // SAFETY: `client_origin` is initialized and the handle is live.
    if !unsafe { ClientToScreen(hwnd, &mut client_origin) }.as_bool() {
        return;
    }
    // Shape the CLIENT area, not the outer window rectangle. An undecorated
    // window still reserves native shadow insets around its client area; a
    // region built from `GetWindowRect` would round the shadow frame while
    // the webview's own corners stayed square 6-7px inside it — the white
    // wedge the titlebar hover used to show.
    let offset_x = client_origin.x - window_rect.left;
    let offset_y = client_origin.y - window_rect.top;
    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;
    // The region bounds are exclusive, and `CreateRoundRectRgn` takes the
    // size of the ellipse its corners are cut from rather than a radius.
    // SAFETY: `hwnd` is a valid live window handle.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
    let ellipse = (radius * scale * 2.0).round() as i32;
    // SAFETY: plain value arguments; the returned handle is checked below.
    let region = unsafe {
        CreateRoundRectRgn(
            offset_x,
            offset_y,
            offset_x + width + 1,
            offset_y + height + 1,
            ellipse,
            ellipse,
        )
    };
    if region.is_invalid() {
        return;
    }
    // SAFETY: the window takes ownership of the region once the call
    // succeeds, so it is only deleted here when it was rejected.
    if unsafe { SetWindowRgn(hwnd, Some(region), true) } == 0 {
        let _ = unsafe { DeleteObject(region.into()) };
    }
}

/// Drops the rounded region so the maximized/fullscreen window can use the
/// whole screen rectangle.
pub fn clear_round_corners(window: &WebviewWindow) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    // SAFETY: `hwnd` is a live window handle. Passing `None` removes the
    // region; the system owns and deletes the previous one.
    let _ = unsafe { SetWindowRgn(hwnd, None, true) };
}
