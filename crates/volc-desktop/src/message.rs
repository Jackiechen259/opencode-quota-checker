use crate::platform::tray::TrayAction;
use iced::window;

/// Every external event and asynchronous result entering the application.
#[derive(Debug, Clone)]
pub enum Message {
    /// The main window finished opening.
    MainWindowOpened(window::Id),
    /// A native close request was received.
    CloseRequested(window::Id),
    /// Poll the tray event bridge.
    PollTray,
    /// A semantic tray action was received.
    Tray(TrayAction),
    /// Close the main window while keeping the daemon alive.
    HideMain,
    /// Stop the daemon and process.
    Exit,
}
