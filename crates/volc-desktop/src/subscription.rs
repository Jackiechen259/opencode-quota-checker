use crate::message::Message;
use crate::App;
use iced::{keyboard, time, window, Subscription};
use std::time::Duration;

/// Builds passive subscriptions from the current state.
pub fn subscription(app: &App) -> Subscription<Message> {
    let mut subscriptions = vec![
        window::close_requests().map(Message::CloseRequested),
        window::events().map(|(id, event)| Message::WindowEvent(id, event)),
        keyboard::listen().map(Message::Keyboard),
    ];
    if app.tray_available() {
        subscriptions.push(time::every(Duration::from_millis(100)).map(|_| Message::PollTray));
    }
    if app.usage().loading {
        subscriptions.push(
            time::every(Duration::from_millis(80))
                .map(|_| Message::Tick(chrono::Utc::now().timestamp_millis())),
        );
    } else if app.has_report() {
        subscriptions.push(
            time::every(Duration::from_secs(1))
                .map(|_| Message::Tick(chrono::Utc::now().timestamp_millis())),
        );
    }
    if let Some(interval) = app.monitor_interval() {
        subscriptions
            .push(time::every(Duration::from_secs(interval)).map(|_| Message::MonitorTick));
    }
    if app.float_position_dirty() {
        subscriptions
            .push(time::every(Duration::from_millis(750)).map(|_| Message::PersistFloatPosition));
    }
    if app.toast_visible() {
        subscriptions.push(time::every(Duration::from_secs(3)).map(|_| Message::DismissToast));
    }
    Subscription::batch(subscriptions)
}
