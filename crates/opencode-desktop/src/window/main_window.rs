use crate::platform::icon;
use iced::{window, Size, Task};

#[cfg(target_os = "windows")]
use raw_window_handle::RawWindowHandle;
#[cfg(target_os = "windows")]
use windows::core::w;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, GetCursorPos, GetPropW, GetWindowLongPtrW, GetWindowRect,
    IsZoomed, RemovePropW, SetPropW, SetWindowLongPtrW, GWLP_WNDPROC, HTBOTTOM, HTBOTTOMLEFT,
    HTBOTTOMRIGHT, HTCLIENT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, WM_NCDESTROY,
    WM_NCHITTEST, WNDPROC,
};

/// Opens the single main window with close requests routed through `Message`.
pub fn open() -> (window::Id, Task<window::Id>) {
    window::open(window::Settings {
        size: Size::new(1_440.0, 900.0),
        min_size: Some(Size::new(680.0, 620.0)),
        icon: Some(icon::window()),
        // The title bar is drawn by the application itself, so the native
        // frame (and its caption buttons) must not be shown.
        decorations: false,
        exit_on_close_request: false,
        ..window::Settings::default()
    })
}

/// Physical-pixel thickness of the invisible resize border around the window.
#[cfg(target_os = "windows")]
const RESIZE_BORDER: i32 = 8;

/// Window-property name under which the original window procedure is stashed.
#[cfg(target_os = "windows")]
const ORIGINAL_WNDPROC_PROP: windows::core::PCWSTR = w!("OpenCodeQuotaChecker.OriginalWndProc");

/// Re-enables edge resizing on the borderless main window.
///
/// winit removes `WS_SIZEBOX` from the non-client area of undecorated windows,
/// so without native help the window could only be resized from the bottom
/// corner the compositor keeps. The window procedure is subclassed instead:
/// `WM_NCHITTEST` reports the classic `HT*` resize zones around the window
/// edges, which makes the system drive the resize loop itself.
///
/// The original procedure is stashed in a window property. `GWLP_USERDATA`
/// must not be touched: winit stores its `WindowData` box there and reads it
/// on every message. Every other message is forwarded untouched, so the
/// mixed-DPI winit patch and the existing window event flow are unaffected.
/// The hook lives for the lifetime of the window, which is recreated on
/// hide-to-tray/show, and `MainWindowOpened` reinstalls it for every new
/// instance; the property is removed when the window is destroyed.
#[cfg(target_os = "windows")]
pub fn install_native_resize(id: window::Id) -> Task<()> {
    window::run(id, |window| {
        let Some(hwnd) = native_hwnd(window) else {
            return;
        };
        // SAFETY: `hwnd` is a valid live window handle.
        let original = unsafe { GetWindowLongPtrW(hwnd, GWLP_WNDPROC) };
        if original == 0 {
            return;
        }
        // SAFETY: `original` was returned by `GetWindowLongPtrW(GWLP_WNDPROC)`
        // and is therefore a valid window procedure (or zero, already handled).
        let original: WNDPROC = unsafe { std::mem::transmute(original) };
        // SAFETY: plain value arguments; the window stays valid for the rest
        // of its lifetime, which is what the subclass expects.
        unsafe {
            let pointer =
                original.map_or(std::ptr::null_mut(), |proc| proc as *mut core::ffi::c_void);
            let _ = SetPropW(hwnd, ORIGINAL_WNDPROC_PROP, Some(HANDLE(pointer)));
            SetWindowLongPtrW(
                hwnd,
                GWLP_WNDPROC,
                borderless_wnd_proc as *const () as isize,
            );
        }
    })
}

/// Window procedure installed on the borderless main window.
///
/// Only `WM_NCHITTEST` is intercepted; everything else goes straight to the
/// original procedure kept in a window property. On `WM_NCDESTROY` the
/// property is removed again; the window is gone right after, so the
/// procedure pointer does not need to be restored.
#[cfg(target_os = "windows")]
unsafe extern "system" fn borderless_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        // SAFETY: `hwnd` is the window the subclass is installed on, and the
        // value written by `install_native_resize` is a valid procedure.
        let original =
            std::mem::transmute::<HANDLE, WNDPROC>(GetPropW(hwnd, ORIGINAL_WNDPROC_PROP));
        if msg == WM_NCHITTEST {
            match resize_hit_test(hwnd) {
                Some(hit) => return LRESULT(hit as isize),
                // Non-edge positions must keep behaving as client area so
                // clicks reach the Iced widgets (title-bar drag included).
                None => return LRESULT(HTCLIENT as isize),
            }
        }
        if msg == WM_NCDESTROY {
            // SAFETY: `hwnd` is a valid live window handle.
            let _ = RemovePropW(hwnd, ORIGINAL_WNDPROC_PROP);
        }
        match original {
            Some(proc) => CallWindowProcW(Some(proc), hwnd, msg, wparam, lparam),
            None => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Resize zone under the cursor, or `None` when it is inside the window.
///
/// Maximized windows must not report resize zones: the system would try to
/// resize against the screen edge instead of restoring the window. While the
/// window is maximized the whole surface stays `HTCLIENT`, so dragging the
/// title bar still restores and moves the window through `HTCAPTION`.
#[cfg(target_os = "windows")]
fn resize_hit_test(hwnd: HWND) -> Option<u32> {
    // SAFETY: `hwnd` is a valid live window handle.
    if unsafe { IsZoomed(hwnd) }.as_bool() {
        return None;
    }
    let mut cursor = POINT::default();
    // SAFETY: the pointer refers to an initialized stack value.
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return None;
    }
    let mut rect = RECT::default();
    // SAFETY: the pointer refers to an initialized stack value and `hwnd` is
    // a valid live window handle.
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return None;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let x = cursor.x - rect.left;
    let y = cursor.y - rect.top;
    let left = x < RESIZE_BORDER;
    let right = x >= width - RESIZE_BORDER;
    let top = y < RESIZE_BORDER;
    let bottom = y >= height - RESIZE_BORDER;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(HTTOPLEFT),
        (true, _, _, true) => Some(HTBOTTOMLEFT),
        (true, _, _, _) => Some(HTLEFT),
        (_, true, true, _) => Some(HTTOPRIGHT),
        (_, true, _, true) => Some(HTBOTTOMRIGHT),
        (_, true, _, _) => Some(HTRIGHT),
        (_, _, true, _) => Some(HTTOP),
        (_, _, _, true) => Some(HTBOTTOM),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn native_hwnd(window: &dyn window::Window) -> Option<HWND> {
    let handle = window.window_handle().ok()?.as_raw();
    match handle {
        RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut _)),
        _ => None,
    }
}
