use crate::message::{Message, SensitiveInput};
use crate::state::CredentialState;
use crate::theme;
use iced::widget::{button, column, container, text, text_input};
use iced::{Element, Fill};
use volc_core::Provider;

/// Renders the no-credential empty state for the active provider.
pub fn view(state: &CredentialState, provider: Provider) -> Element<'_, Message> {
    let form = match provider {
        Provider::VolcArkV => ark_form(state),
        Provider::OpenCodeGo => opencode_form(state),
    };

    let mut content = column![container(form)
        .width(Fill)
        .padding(28)
        .style(move |_| theme::card())]
    .spacing(16);
    if let Some(error) = &state.error {
        content = content.push(
            container(text(&error.user).color(theme::palette::TEXT_PRIMARY))
                .width(Fill)
                .padding([10, 14])
                .style(move |_| theme::danger_box()),
        );
    }
    content.into()
}

/// Volcano Ark Access Key / Secret Key form.
fn ark_form(state: &CredentialState) -> Element<'_, Message> {
    let access_key = text_input("Access Key", &state.access_key)
        .on_input(|value| Message::AccessKeyChanged(SensitiveInput(value)))
        .padding(12);
    let secret_key = text_input("Secret Key", &state.secret_key)
        .on_input(|value| Message::SecretKeyChanged(SensitiveInput(value)))
        .secure(true)
        .padding(12);
    let can_save = !state.mutating
        && !state.access_key.trim().is_empty()
        && !state.secret_key.trim().is_empty();
    let save = save_button("保存到系统钥匙串", can_save, Message::SaveCredentials);

    column![
        text("尚未配置访问凭证")
            .size(22)
            .color(theme::palette::TEXT_PRIMARY),
        text("配置火山方舟凭证后即可查看配额使用情况。AK/SK 仅保存到操作系统钥匙串，不写入普通配置或日志。")
            .size(13)
            .color(theme::palette::TEXT_MUTED),
        access_key,
        secret_key,
        save,
    ]
    .spacing(14)
    .into()
}

/// OpenCode Go Workspace ID + auth cookie form.
fn opencode_form(state: &CredentialState) -> Element<'_, Message> {
    let workspace = text_input("Workspace ID", &state.opencode_workspace)
        .on_input(Message::OpenCodeWorkspaceChanged)
        .padding(12);
    let cookie = text_input("Auth Cookie", &state.opencode_cookie)
        .on_input(|value| Message::OpenCodeCookieChanged(SensitiveInput(value)))
        .secure(true)
        .padding(12);
    let can_save = !state.mutating
        && !state.opencode_workspace.trim().is_empty()
        && !state.opencode_cookie.trim().is_empty();
    let save = save_button(
        "保存到系统钥匙串",
        can_save,
        Message::SaveOpenCodeCredentials,
    );

    column![
        text("尚未配置 OpenCode Go")
            .size(22)
            .color(theme::palette::TEXT_PRIMARY),
        text("OpenCode Go 尚无公开的配额 API，配额数据来自登录后的工作区面板。Workspace ID 保存在普通配置中，Auth Cookie 仅保存到系统钥匙串。")
            .size(13)
            .color(theme::palette::TEXT_MUTED),
        text("获取方式：登录 opencode.ai → 打开 OpenCode Go 工作区 → 从地址栏复制 Workspace ID → 在浏览器开发者工具中找到 opencode.ai 的 auth Cookie 值。")
            .size(12)
            .color(theme::palette::TEXT_MUTED),
        workspace,
        cookie,
        text("请将 Auth Cookie 视为密码保管。它会随请求发送到 opencode.ai，不会写入配置或日志。")
            .size(12)
            .color(theme::palette::WARNING),
        save,
    ]
    .spacing(14)
    .into()
}

fn save_button(
    label: &'static str,
    enabled: bool,
    message: Message,
) -> iced::widget::Button<'static, Message> {
    if enabled {
        button(label)
            .on_press(message)
            .style(button::primary)
            .padding([10, 20])
    } else {
        button(label).padding([10, 20])
    }
}
