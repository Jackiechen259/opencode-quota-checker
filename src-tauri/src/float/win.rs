//! Native Windows adapters for the borderless floating window.
//!
//! Physical-pixel work-area queries, position restore clamped to the work area,
//! and the drag-state probe used by the deferred side un-dock. Edge snapping
//! itself is platform-neutral and lives in [`crate::float::controller`];
//! rounded-window shaping lives in the shared [`crate::window::win`] helper
//! because the main window needs the same native region.

use crate::config::FloatPosition;
use crate::window::win::hwnd;
use tauri::WebviewWindow;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
};

/// Returns the work area of the monitor under the window as
/// `(left, top, right, bottom)` in physical (virtual-desktop) pixels.
///
/// The rectangle excludes the taskbar and other appbars, which is what every
/// docking threshold and snap is measured against.
pub fn work_area(window: &WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    let hwnd = hwnd(window)?;
    native_rects(hwnd).map(|(_, work_rect)| {
        (
            work_rect.left,
            work_rect.top,
            work_rect.right,
            work_rect.bottom,
        )
    })
}

/// Whether the primary mouse button is still held down.
///
/// The dock-settle task uses this to keep a side rail collapsed until the
/// native move loop has ended: while the button is down the move loop owns the
/// window position, so expanding a right-edge rail mid-drag would leave most of
/// the card off-screen (and the loop would undo any repositioning anyway).
pub fn primary_button_down() -> bool {
    // SAFETY: `GetAsyncKeyState` only reads the current key state and has no
    // preconditions.
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

/// Restores a saved physical position, clamped to the nearest monitor work
/// area (so a monitor that disappeared cannot strand the window off-screen).
pub fn restore_position(window: &WebviewWindow, position: FloatPosition) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    let Some((window_rect, _)) = native_rects(hwnd) else {
        return;
    };
    let width = window_rect.right - window_rect.left;
    let height = window_rect.bottom - window_rect.top;
    let target = POINT {
        x: position.x.saturating_add(width / 2),
        y: position.y.saturating_add(height / 2),
    };
    // SAFETY: the point is a plain value and the returned monitor handle is
    // only used immediately to query its work area.
    let monitor = unsafe { MonitorFromPoint(target, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..MONITORINFO::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return;
    }
    let maximum_x = (info.rcWork.right - width).max(info.rcWork.left);
    let maximum_y = (info.rcWork.bottom - height).max(info.rcWork.top);
    let x = position.x.clamp(info.rcWork.left, maximum_x);
    let y = position.y.clamp(info.rcWork.top, maximum_y);
    // SAFETY: `hwnd` belongs to the live window and the flags keep its size,
    // z-order, and activation state unchanged.
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
}

fn native_rects(hwnd: HWND) -> Option<(RECT, RECT)> {
    let mut window_rect = RECT::default();
    // SAFETY: both pointers refer to initialized stack values valid for the
    // duration of each API call.
    unsafe { GetWindowRect(hwnd, &mut window_rect) }.ok()?;
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..MONITORINFO::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return None;
    }
    Some((window_rect, info.rcWork))
}
