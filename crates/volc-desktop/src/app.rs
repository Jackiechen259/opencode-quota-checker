use crate::message::Message;
use crate::platform::tray::{TrayAction, TrayService};
use crate::state::WindowState;
use crate::{subscription, view, window as app_window};
use iced::{window, Element, Subscription, Task, Theme};

/// The single state container shared by every application window.
pub struct App {
    windows: WindowState,
    tray: Option<TrayService>,
    tray_error: Option<String>,
}

impl App {
    /// Initializes platform services and opens the main window.
    pub fn boot() -> (Self, Task<Message>) {
        let (tray, tray_error) = match TrayService::new() {
            Ok(tray) => (Some(tray), None),
            Err(error) => {
                tracing::error!(%error, "tray initialization failed; close behavior is Exit");
                (None, Some(error))
            }
        };
        let (id, open) = app_window::main_window::open();
        let mut windows = WindowState::default();
        windows.set_main(id);

        (
            Self {
                windows,
                tray,
                tray_error,
            },
            open.map(Message::MainWindowOpened),
        )
    }

    /// Applies one message and returns any asynchronous runtime work.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainWindowOpened(id) => {
                self.windows.set_main(id);
                Task::none()
            }
            Message::CloseRequested(id) if self.windows.main() == Some(id) => self.hide_main(),
            Message::CloseRequested(_) => Task::none(),
            Message::PollTray => self
                .tray
                .as_ref()
                .and_then(TrayService::try_recv)
                .map_or_else(Task::none, |action| self.update(Message::Tray(action))),
            Message::Tray(TrayAction::ShowMain) => self.show_main(),
            Message::Tray(TrayAction::HideMain) | Message::HideMain => self.hide_main(),
            Message::Tray(TrayAction::Quit) | Message::Exit => iced::exit(),
        }
    }

    /// Renders the requested window from shared state.
    pub fn view(&self, _id: window::Id) -> Element<'_, Message> {
        view::main(self)
    }

    /// Returns the title for a native window.
    pub fn title(&self, _id: window::Id) -> String {
        "VOLC Status".to_owned()
    }

    /// Returns passive runtime subscriptions.
    pub fn subscription(&self) -> Subscription<Message> {
        subscription::subscription(self)
    }

    /// Selects the built-in dark theme.
    pub fn theme(&self, _id: window::Id) -> Option<Theme> {
        Some(Theme::Dark)
    }

    /// Reports whether the native tray is usable.
    pub fn tray_available(&self) -> bool {
        self.tray.is_some()
    }

    /// Returns a bounded, non-sensitive tray initialization error.
    pub fn tray_error(&self) -> Option<&str> {
        self.tray_error.as_deref()
    }

    fn show_main(&mut self) -> Task<Message> {
        if let Some(id) = self.windows.main() {
            window::gain_focus(id)
        } else {
            let (id, open) = app_window::main_window::open();
            self.windows.set_main(id);
            open.map(Message::MainWindowOpened)
        }
    }

    fn hide_main(&mut self) -> Task<Message> {
        let Some(id) = self.windows.take_main() else {
            return Task::none();
        };
        if self.tray.is_some() {
            window::close(id)
        } else {
            iced::exit()
        }
    }
}
