use iced::window;

/// IDs of all windows owned by the application state machine.
#[derive(Debug, Default)]
pub struct WindowState {
    main: Option<window::Id>,
}

impl WindowState {
    /// Returns the current main-window ID.
    pub fn main(&self) -> Option<window::Id> {
        self.main
    }

    /// Records the only main-window instance.
    pub fn set_main(&mut self, id: window::Id) {
        self.main = Some(id);
    }

    /// Removes and returns the main-window ID.
    pub fn take_main(&mut self) -> Option<window::Id> {
        self.main.take()
    }
}
