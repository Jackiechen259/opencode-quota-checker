mod app;
mod config;
mod message;
mod opencode_login;
mod platform;
mod state;
mod subscription;
mod theme;
mod view;
mod window;

use app::App;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "volc_desktop=info,volc_core=info".into()),
        )
        .init();

    iced::daemon(App::boot, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .antialiasing(true)
        .run()
}
