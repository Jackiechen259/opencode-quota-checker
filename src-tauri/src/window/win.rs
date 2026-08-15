//! Native Windows adapters for the borderless windows.
//!
//! Ported from the archived Iced `window/float_window.rs` and
//! `window/main_window.rs`: rounded window regions, physical-pixel position
//! restore clamped to the monitor work area, and snapping to the monitor top.

use crate::config::FloatPosition;
use raw_window_handle::HasWindowHandle;
use tauri::WebviewWindow;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, DeleteObject, GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow,
    SetWindowRgn, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
};

/// Corner radius of the floating card, in logical pixels.
const CORNER_RADIUS: f32 = 16.0;

/// Returns the native `HWND` of a live window.
fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    let handle = window.window_handle().ok()?.as_raw();
    match handle {
        raw_window_handle::RawWindowHandle::Win32(handle) => {
            Some(HWND(handle.hwnd.get() as *mut _))
        }
        _ => None,
    }
}

/// Clips the undecorated floating window to a rounded rectangle.
///
/// The webview surface is opaque on Windows, so the renderer cannot draw the
/// rounded shape itself. A window region is what remains. The region lives in
/// window coordinates and does not follow a resize, so it is reapplied on
/// every resize/DPI change (the float window's `Resized` handler calls this).
pub fn round_corners(window: &WebviewWindow) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    let mut rect = RECT::default();
    // SAFETY: `hwnd` is a valid live window handle and the pointer refers to
    // an initialized stack value.
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return;
    }
    // SAFETY: `hwnd` is a valid live window handle.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
    // The region bounds are exclusive, and `CreateRoundRectRgn` takes the
    // size of the ellipse its corners are cut from rather than a radius.
    let ellipse = (CORNER_RADIUS * scale * 2.0).round() as i32;
    // SAFETY: plain value arguments; the returned handle is checked below.
    let region = unsafe {
        CreateRoundRectRgn(
            0,
            0,
            rect.right - rect.left + 1,
            rect.bottom - rect.top + 1,
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

/// Returns the work-area top of the monitor under the window, in physical
/// (virtual-desktop) pixels.
pub fn work_area_top(window: &WebviewWindow) -> Option<i32> {
    let hwnd = hwnd(window)?;
    native_rects(hwnd).map(|(_, work_rect)| work_rect.top)
}

/// Snaps the window to the top of its current monitor work area.
pub fn snap_to_monitor_top(window: &WebviewWindow) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    let Some((window_rect, work_rect)) = native_rects(hwnd) else {
        return;
    };
    // SAFETY: `hwnd` belongs to the live window and the flags keep its size,
    // z-order, and activation state unchanged.
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            None,
            window_rect.left,
            work_rect.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
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
