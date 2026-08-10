// Release builds on Windows are GUI applications: launching them must not
// open a console window. Debug builds keep the console so development logs
// from `tracing_subscriber` remain visible under `cargo run`.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod config;
mod message;
mod opencode_login;
mod platform;
mod state;
mod subscription;
mod theme;
mod update;
mod view;
mod window;

use app::App;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "opencode_desktop=info,opencode_core=info".into()),
        )
        .init();

    iced::daemon(App::boot, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .font(include_bytes!("../assets/fonts/NotoSansSC[wght].ttf").as_slice())
        .default_font(theme::typography::ui())
        .style(App::window_style)
        .antialiasing(true)
        .run()
}
