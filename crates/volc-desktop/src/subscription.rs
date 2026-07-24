use crate::message::Message;
use crate::App;
use iced::{time, window, Subscription};
use std::time::Duration;

/// Builds passive subscriptions from the current state.
pub fn subscription(app: &App) -> Subscription<Message> {
    let mut subscriptions = vec![window::close_requests().map(Message::CloseRequested)];
    if app.tray_available() {
        subscriptions.push(time::every(Duration::from_millis(100)).map(|_| Message::PollTray));
    }
    if app.has_report() {
        subscriptions.push(
            time::every(Duration::from_secs(1))
                .map(|_| Message::Tick(chrono::Utc::now().timestamp_millis())),
        );
    }
    if let Some(interval) = app.monitor_interval() {
        subscriptions
            .push(time::every(Duration::from_secs(interval)).map(|_| Message::MonitorTick));
    }
    Subscription::batch(subscriptions)
}
