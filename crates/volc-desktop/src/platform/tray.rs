use std::sync::mpsc::{self, Receiver};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Semantic application actions exposed by the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Open or focus the main window.
    ShowMain,
    /// Close the main window while retaining the daemon.
    HideMain,
    /// Terminate the process.
    Quit,
}

/// Owns the tray icon, menu items, and event receiver for their full lifetime.
pub struct TrayService {
    _tray_icon: TrayIcon,
    _open_item: MenuItem,
    _hide_item: MenuItem,
    _quit_item: MenuItem,
    receiver: Receiver<TrayAction>,
}

impl TrayService {
    /// Creates the native tray menu and its channel bridge.
    pub fn new() -> Result<Self, String> {
        let menu = Menu::new();
        let open_item = MenuItem::new("打开主窗口", true, None);
        let hide_item = MenuItem::new("隐藏主窗口", true, None);
        let quit_item = MenuItem::new("退出", true, None);
        menu.append_items(&[
            &open_item,
            &hide_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])
        .map_err(|error| error.to_string())?;

        let open_id = open_item.id().clone();
        let hide_id = hide_item.id().clone();
        let quit_id = quit_item.id().clone();
        let (sender, receiver) = mpsc::channel();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = if event.id == open_id {
                Some(TrayAction::ShowMain)
            } else if event.id == hide_id {
                Some(TrayAction::HideMain)
            } else if event.id == quit_id {
                Some(TrayAction::Quit)
            } else {
                None
            };
            if let Some(action) = action {
                let _ = sender.send(action);
            }
        }));

        let icon = Icon::from_rgba([0x2f, 0x80, 0xed, 0xff].repeat(16 * 16), 16, 16)
            .map_err(|error| error.to_string())?;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("VOLC Status")
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .build()
            .map_err(|error| error.to_string())?;

        Ok(Self {
            _tray_icon: tray_icon,
            _open_item: open_item,
            _hide_item: hide_item,
            _quit_item: quit_item,
            receiver,
        })
    }

    /// Returns the next queued tray action without blocking the UI thread.
    pub fn try_recv(&self) -> Option<TrayAction> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for TrayService {
    fn drop(&mut self) {
        MenuEvent::set_event_handler::<fn(MenuEvent)>(None);
    }
}
