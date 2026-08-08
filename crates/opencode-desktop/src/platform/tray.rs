use crate::platform::icon;
use std::sync::mpsc::{self, Receiver};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

/// Semantic application actions exposed by the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Open or focus the main window.
    ShowMain,
    /// Close the main window while retaining the daemon.
    HideMain,
    /// Open or close the floating window.
    ToggleFloat,
    /// Terminate the process.
    Quit,
}

/// Owns the tray icon, menu items, and event receiver for their full lifetime.
pub struct TrayService {
    _tray_icon: TrayIcon,
    _open_item: MenuItem,
    _hide_item: MenuItem,
    float_item: CheckMenuItem,
    _quit_item: MenuItem,
    receiver: Receiver<TrayAction>,
}

impl TrayService {
    /// Creates the native tray menu and its channel bridge.
    pub fn new() -> Result<Self, String> {
        let menu = Menu::new();
        let open_item = MenuItem::new("打开主窗口", true, None);
        let hide_item = MenuItem::new("隐藏主窗口", true, None);
        let float_item = CheckMenuItem::new("显示悬浮窗", true, false, None);
        let quit_item = MenuItem::new("退出", true, None);
        menu.append_items(&[
            &open_item,
            &hide_item,
            &float_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])
        .map_err(|error| error.to_string())?;

        let open_id = open_item.id().clone();
        let hide_id = hide_item.id().clone();
        let float_id = float_item.id().clone();
        let quit_id = quit_item.id().clone();
        let (sender, receiver) = mpsc::channel();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = if event.id == open_id {
                Some(TrayAction::ShowMain)
            } else if event.id == hide_id {
                Some(TrayAction::HideMain)
            } else if event.id == float_id {
                Some(TrayAction::ToggleFloat)
            } else if event.id == quit_id {
                Some(TrayAction::Quit)
            } else {
                None
            };
            if let Some(action) = action {
                let _ = sender.send(action);
            }
        }));

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("OpenCode Quota Checker")
            .with_menu(Box::new(menu))
            .with_icon(icon::tray()?)
            .build()
            .map_err(|error| error.to_string())?;

        Ok(Self {
            _tray_icon: tray_icon,
            _open_item: open_item,
            _hide_item: hide_item,
            float_item,
            _quit_item: quit_item,
            receiver,
        })
    }

    /// Returns the next queued tray action without blocking the UI thread.
    pub fn try_recv(&self) -> Option<TrayAction> {
        self.receiver.try_recv().ok()
    }

    /// Synchronizes the floating-window check mark.
    pub fn set_float_open(&self, open: bool) {
        self.float_item.set_checked(open);
    }
}

impl Drop for TrayService {
    fn drop(&mut self) {
        MenuEvent::set_event_handler::<fn(MenuEvent)>(None);
    }
}
