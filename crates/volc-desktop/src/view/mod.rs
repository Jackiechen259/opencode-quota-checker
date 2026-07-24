mod credentials;

use crate::message::Message;
use crate::App;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill, Font};

/// Renders the native application shell and current business state.
pub fn main(app: &App) -> Element<'_, Message> {
    let tray_status = app.tray_error().map_or_else(
        || "系统托盘已连接；关闭窗口后仍可从托盘恢复。".to_owned(),
        |error| format!("系统托盘不可用；关闭窗口将退出：{error}"),
    );
    let header = row![
        column![
            text("VOLC Status").size(32),
            text("火山方舟 Agent Plan AFP 配额监控")
        ]
        .spacing(4),
        button("隐藏到托盘").on_press(Message::HideMain),
        button("退出").on_press(Message::Exit),
    ]
    .spacing(12);
    let body: Element<'_, Message> = if app.credentials().checking {
        text("正在检查系统钥匙串…").into()
    } else if !app.credentials().configured {
        credentials::view(app.credentials())
    } else {
        usage(app)
    };
    let content = column![header, text(tray_status), body]
        .spacing(20)
        .padding(28);

    container(content).width(Fill).height(Fill).into()
}

fn usage(app: &App) -> Element<'_, Message> {
    let state = app.usage();
    let refresh = if state.loading {
        button("刷新中…")
    } else {
        button("刷新").on_press(Message::Refresh)
    };
    let raw = if state.raw_loading {
        button("读取原始响应中…")
    } else {
        button("查看原始 JSON").on_press(Message::LoadRaw)
    };
    let actions = row![
        refresh,
        raw,
        button("删除凭证").on_press(Message::ClearCredentials)
    ]
    .spacing(10);
    let mut content = column![actions].spacing(14);

    if let Some(report) = &state.report {
        content = content.push(text(format!(
            "{} 套餐 · {} 个配额窗口 · 更新于 {}",
            report.plan_type,
            report.windows.len(),
            report.fetched_at
        )));
    } else if state.loading {
        content = content.push(text("正在加载用量…"));
    } else {
        content = content.push(text("尚无用量数据。"));
    }
    if let Some(error) = &state.error {
        content = content
            .push(text(&error.user))
            .push(text(format!("技术详情：{}", error.detail)).size(12));
    }
    if let Some(raw) = &state.raw {
        content = content.push(
            scrollable(text(raw).font(Font::MONOSPACE).size(13)).height(iced::Length::Fixed(320.0)),
        );
    }
    content.into()
}
