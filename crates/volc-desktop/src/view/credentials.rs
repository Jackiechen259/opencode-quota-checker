use crate::message::{Message, SensitiveInput};
use crate::state::CredentialState;
use crate::theme;
use iced::widget::{button, column, container, text, text_input};
use iced::{Element, Fill};

/// Renders the system-keyring credential form (no-credential empty state).
pub fn view(state: &CredentialState) -> Element<'_, Message> {
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
    let save = if can_save {
        button("保存到系统钥匙串")
            .on_press(Message::SaveCredentials)
            .style(button::primary)
            .padding([10, 20])
    } else {
        button("保存到系统钥匙串").padding([10, 20])
    };
    let form = column![
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
    .spacing(14);

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
