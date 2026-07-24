use crate::message::{Message, SensitiveInput};
use crate::state::CredentialState;
use iced::widget::{button, column, text, text_input};
use iced::Element;

/// Renders the system-keyring credential form.
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
        button("保存到系统钥匙串").on_press(Message::SaveCredentials)
    } else {
        button("保存到系统钥匙串")
    };
    let mut content = column![
        text("配置访问凭证").size(26),
        text("AK/SK 仅保存到操作系统钥匙串，不写入普通配置或日志。"),
        access_key,
        secret_key,
        save,
    ]
    .spacing(14);
    if let Some(error) = &state.error {
        content = content.push(text(&error.user));
    }
    content.into()
}
