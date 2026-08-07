use iced::widget::{button, column, text};
use iced::{time, window, Element, Size, Subscription, Task};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[derive(Debug, Clone, Copy)]
enum TrayAction {
    ShowMain,
    HideMain,
    Quit,
}

struct TrayService {
    _tray_icon: TrayIcon,
    _open_item: MenuItem,
    _hide_item: MenuItem,
    _quit_item: MenuItem,
    receiver: Receiver<TrayAction>,
}

impl TrayService {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        let open_item = MenuItem::new("打开主窗口", true, None);
        let hide_item = MenuItem::new("隐藏主窗口", true, None);
        let quit_item = MenuItem::new("退出", true, None);
        menu.append_items(&[
            &open_item,
            &hide_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])?;

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

        let icon = Icon::from_rgba(vec![0x2f, 0x80, 0xed, 0xff].repeat(16 * 16), 16, 16)?;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("VOLC Status Iced tray spike")
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            _open_item: open_item,
            _hide_item: hide_item,
            _quit_item: quit_item,
            receiver,
        })
    }

    fn try_recv(&self) -> Option<TrayAction> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for TrayService {
    fn drop(&mut self) {
        MenuEvent::set_event_handler::<fn(MenuEvent)>(None);
    }
}

#[derive(Default)]
struct Spike {
    main_window: Option<window::Id>,
    tray: Option<TrayService>,
    tray_error: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    MainWindowOpened(window::Id),
    CloseRequested(window::Id),
    PollTray,
    HideMain,
    Quit,
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "iced_tray_daemon_spike=info".into()),
        )
        .init();

    iced::daemon(Spike::boot, Spike::update, Spike::view)
        .title(title)
        .subscription(Spike::subscription)
        .run()
}

fn title(_state: &Spike, _id: window::Id) -> String {
    "Iced tray daemon spike".to_owned()
}

impl Spike {
    fn boot() -> (Self, Task<Message>) {
        let (tray, tray_error) = match TrayService::new() {
            Ok(tray) => (Some(tray), None),
            Err(error) => {
                tracing::error!(%error, "tray initialization failed; close will exit");
                (None, Some(error.to_string()))
            }
        };
        let (id, open) = open_main_window();

        (
            Self {
                main_window: Some(id),
                tray,
                tray_error,
            },
            open.map(Message::MainWindowOpened),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainWindowOpened(id) => {
                self.main_window = Some(id);
                Task::none()
            }
            Message::CloseRequested(id) if self.main_window == Some(id) => self.hide_main(),
            Message::CloseRequested(_) => Task::none(),
            Message::PollTray => match self.tray.as_ref().and_then(TrayService::try_recv) {
                Some(TrayAction::ShowMain) => self.show_main(),
                Some(TrayAction::HideMain) => self.hide_main(),
                Some(TrayAction::Quit) => iced::exit(),
                None => Task::none(),
            },
            Message::HideMain => self.hide_main(),
            Message::Quit => iced::exit(),
        }
    }

    fn view(&self, _id: window::Id) -> Element<'_, Message> {
        let tray_status = self.tray_error.as_deref().map_or_else(
            || "托盘已初始化。关闭此窗口后进程继续运行。".to_owned(),
            |error| format!("托盘不可用，关闭窗口将退出：{error}"),
        );

        column![
            text("Iced daemon + tray lifecycle spike").size(24),
            text(tray_status),
            button("隐藏主窗口").on_press(Message::HideMain),
            button("退出进程").on_press(Message::Quit),
        ]
        .padding(24)
        .spacing(16)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::close_requests().map(Message::CloseRequested),
            time::every(Duration::from_millis(100)).map(|_| Message::PollTray),
        ])
    }

    fn show_main(&mut self) -> Task<Message> {
        if let Some(id) = self.main_window {
            window::gain_focus(id)
        } else {
            let (id, open) = open_main_window();
            self.main_window = Some(id);
            open.map(Message::MainWindowOpened)
        }
    }

    fn hide_main(&mut self) -> Task<Message> {
        let Some(id) = self.main_window.take() else {
            return Task::none();
        };

        if self.tray.is_some() {
            window::close(id)
        } else {
            iced::exit()
        }
    }
}

fn open_main_window() -> (window::Id, Task<window::Id>) {
    window::open(window::Settings {
        size: Size::new(520.0, 240.0),
        min_size: Some(Size::new(420.0, 200.0)),
        exit_on_close_request: false,
        ..window::Settings::default()
    })
}
