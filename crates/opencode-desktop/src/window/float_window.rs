use crate::config::{FloatMode, FloatPosition};
use crate::platform::icon;
use iced::{window, Point, Size, Task};

#[cfg(target_os = "windows")]
use raw_window_handle::RawWindowHandle;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, POINT, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    CreateRoundRectRgn, DeleteObject, GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow,
    SetWindowRgn, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::HiDpi::GetDpiForWindow;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
};

/// Distance from the monitor top that activates the compact dock.
pub const TOP_SNAP_DISTANCE: f32 = 18.0;
/// Larger release distance prevents repeated mode changes near the boundary.
pub const TOP_RELEASE_DISTANCE: f32 = 24.0;

/// Native, physical-pixel window geometry used on Windows.
///
/// Iced reports a moved window in logical pixels relative to the window's
/// current DPI. Those coordinates are discontinuous when a window crosses
/// monitors with different scale factors, so Windows persistence and docking
/// must use the native virtual-desktop coordinate space instead.
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub struct WindowGeometry {
    pub position: Point,
    pub monitor_top: f32,
    pub scale_factor: f32,
}

/// Opens the unique always-on-top, borderless floating window.
pub fn open(mode: FloatMode, position: Option<FloatPosition>) -> (window::Id, Task<window::Id>) {
    #[cfg(target_os = "windows")]
    let position = {
        // The saved coordinates are physical pixels on Windows. Iced's
        // `Specific` variant treats them as logical pixels, so restoration is
        // performed through SetWindowPos after the HWND has been created.
        let _ = position;
        window::Position::Default
    };
    #[cfg(not(target_os = "windows"))]
    let position = position.map_or(window::Position::Default, |position| {
        window::Position::Specific(Point::new(position.x as f32, position.y as f32))
    });
    window::open(window::Settings {
        size: mode.size(),
        min_size: Some(Size::new(200.0, 40.0)),
        decorations: false,
        // The DX12 swapchain only advertises `CompositeAlphaMode::Opaque`, so a
        // transparent surface renders as opaque black on Windows. The window is
        // painted edge to edge with the card color instead and DWM rounds it.
        transparent: cfg!(not(target_os = "windows")),
        resizable: false,
        level: window::Level::AlwaysOnTop,
        position,
        icon: Some(icon::window()),
        exit_on_close_request: false,
        ..window::Settings::default()
    })
}

/// Returns whether the window should use its top-docked presentation.
///
/// `monitor_top` and `position_y` are logical coordinates. The larger release
/// threshold provides hysteresis while the user drags along the screen edge.
pub fn is_top_docked(currently_docked: bool, position_y: f32, monitor_top: f32) -> bool {
    is_top_docked_at_scale(currently_docked, position_y, monitor_top, 1.0)
}

/// DPI-aware variant of [`is_top_docked`].
pub fn is_top_docked_at_scale(
    currently_docked: bool,
    position_y: f32,
    monitor_top: f32,
    scale_factor: f32,
) -> bool {
    let distance = position_y - monitor_top;
    let scale_factor = scale_factor.max(0.01);
    if distance < -TOP_RELEASE_DISTANCE * scale_factor {
        false
    } else if currently_docked {
        distance <= TOP_RELEASE_DISTANCE * scale_factor
    } else {
        distance <= TOP_SNAP_DISTANCE * scale_factor
    }
}

/// Reads the physical window position and current monitor work area.
#[cfg(target_os = "windows")]
pub fn geometry(id: window::Id) -> Task<Option<WindowGeometry>> {
    window::run(id, native_geometry)
}

/// Restores a saved physical position, clamped to the nearest monitor work area.
#[cfg(target_os = "windows")]
pub fn restore_position(
    id: window::Id,
    position: Option<FloatPosition>,
) -> Task<Option<WindowGeometry>> {
    window::run(id, move |window| {
        let hwnd = native_hwnd(window)?;
        if let Some(position) = position {
            native_set_position(hwnd, position)?;
        }
        native_geometry(window)
    })
}

/// Corner radius of the floating card, in logical pixels.
#[cfg(target_os = "windows")]
const CORNER_RADIUS: f32 = 16.0;

/// Clips the undecorated floating window to a rounded rectangle.
///
/// The window surface is opaque on Windows, so the renderer cannot draw the
/// rounded shape itself. `DWMWA_WINDOW_CORNER_PREFERENCE` is no help either:
/// DWM ignores it for a borderless popup like this one, even with non-client
/// rendering forced on. A window region is what remains, and unlike a DWM
/// frame it rounds the window without also giving it a drop shadow.
///
/// The region lives in window coordinates and does not follow a resize, so it
/// has to be reapplied whenever the window changes size or DPI.
#[cfg(target_os = "windows")]
pub fn round_corners(id: window::Id) -> Task<()> {
    window::run(id, |window| {
        let Some(hwnd) = native_hwnd(window) else {
            return;
        };
        let mut rect = RECT::default();
        // SAFETY: `hwnd` is a valid live window handle and the pointer refers
        // to an initialized stack value.
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
    })
}

/// Snaps the native window to the top of its current monitor work area.
#[cfg(target_os = "windows")]
pub fn snap_to_monitor_top(id: window::Id) -> Task<()> {
    window::run(id, |window| {
        let Some(hwnd) = native_hwnd(window) else {
            return;
        };
        let Some((window_rect, work_rect)) = native_rects(hwnd) else {
            return;
        };
        // SAFETY: `hwnd` belongs to the live Iced window and the flags keep its
        // size, z-order, and activation state unchanged.
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
    })
}

#[cfg(target_os = "windows")]
fn native_geometry(window: &dyn window::Window) -> Option<WindowGeometry> {
    let hwnd = native_hwnd(window)?;
    let (window_rect, work_rect) = native_rects(hwnd)?;
    // SAFETY: `hwnd` is a valid live window handle.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    Some(WindowGeometry {
        position: Point::new(window_rect.left as f32, window_rect.top as f32),
        monitor_top: work_rect.top as f32,
        scale_factor: if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 },
    })
}

#[cfg(target_os = "windows")]
fn native_hwnd(window: &dyn window::Window) -> Option<HWND> {
    let handle = window.window_handle().ok()?.as_raw();
    match handle {
        RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut _)),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
fn native_set_position(hwnd: HWND, position: FloatPosition) -> Option<()> {
    let (window_rect, _) = native_rects(hwnd)?;
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
        return None;
    }
    let maximum_x = (info.rcWork.right - width).max(info.rcWork.left);
    let maximum_y = (info.rcWork.bottom - height).max(info.rcWork.top);
    let x = position.x.clamp(info.rcWork.left, maximum_x);
    let y = position.y.clamp(info.rcWork.top, maximum_y);
    // SAFETY: `hwnd` belongs to the live Iced window and the flags keep its
    // size, z-order, and activation state unchanged.
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    }
    .ok()
}

/// Clamps a floating position to the visible logical bounds of one monitor.
#[cfg(any(not(target_os = "windows"), test))]
pub fn clamp_position(position: Point, monitor: Size, window_size: Size) -> Point {
    let maximum_x = (monitor.width - window_size.width).max(0.0);
    let maximum_y = (monitor.height - window_size.height).max(0.0);
    Point::new(
        position.x.clamp(0.0, maximum_x),
        position.y.clamp(0.0, maximum_y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offscreen_positions_are_moved_into_view() {
        assert_eq!(
            clamp_position(
                Point::new(-500.0, 2_000.0),
                Size::new(1_920.0, 1_080.0),
                Size::new(344.0, 404.0)
            ),
            Point::new(0.0, 676.0)
        );
    }

    #[test]
    fn top_snap_uses_hysteresis() {
        assert!(is_top_docked(false, 18.0, 0.0));
        assert!(!is_top_docked(false, 19.0, 0.0));
        assert!(is_top_docked(true, 24.0, 0.0));
        assert!(!is_top_docked(true, 25.0, 0.0));
        assert!(!is_top_docked(false, -25.0, 0.0));
    }

    #[test]
    fn top_snap_accepts_non_zero_monitor_origins() {
        assert!(is_top_docked(false, -1_062.0, -1_080.0));
        assert!(!is_top_docked(false, -1_061.0, -1_080.0));
    }

    #[test]
    fn top_snap_threshold_scales_with_monitor_dpi() {
        assert!(is_top_docked_at_scale(false, 27.0, 0.0, 1.5));
        assert!(!is_top_docked_at_scale(false, 28.0, 0.0, 1.5));
        assert!(is_top_docked_at_scale(true, 36.0, 0.0, 1.5));
        assert!(!is_top_docked_at_scale(true, 37.0, 0.0, 1.5));
    }
}
