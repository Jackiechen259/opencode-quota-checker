use crate::message::Message;
use crate::App;
use iced::{time, window, Subscription};
use std::time::Duration;

/// Builds passive subscriptions from the current state.
pub fn subscription(app: &App) -> Subscription<Message> {
    let close_requests = window::close_requests().map(Message::CloseRequested);
    if app.tray_available() {
        Subscription::batch([
            close_requests,
            time::every(Duration::from_millis(100)).map(|_| Message::PollTray),
        ])
    } else {
        close_requests
    }
}
