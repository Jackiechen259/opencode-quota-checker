use crate::state::UiError;
use opencode_core::{AlertDecision, OpenCodeError};

/// Delivers threshold decisions using the platform notification service.
pub fn deliver(decisions: Vec<AlertDecision>) -> Result<(), UiError> {
    for decision in decisions {
        send(&decision.title, &decision.body).map_err(notification_error)?;
    }
    Ok(())
}

fn notification_error(error: impl std::fmt::Display) -> UiError {
    UiError::from(OpenCodeError::Config(format!(
        "desktop notification failed: {error}"
    )))
}

#[cfg(windows)]
fn send(title: &str, body: &str) -> windows::core::Result<()> {
    use windows::{
        core::HSTRING,
        Data::Xml::Dom::XmlDocument,
        Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
        UI::Notifications::{ToastNotification, ToastNotificationManager},
    };

    const POWERSHELL_APP_ID: &str =
        r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe";

    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
    let _apartment = WinRtApartment;
    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\">\
         <text>{}</text><text>{}</text>\
         </binding></visual></toast>",
        escape_xml(title),
        escape_xml(body)
    );
    let document = XmlDocument::new()?;
    document.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&document)?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(POWERSHELL_APP_ID))?
        .Show(&toast)
}

#[cfg(windows)]
struct WinRtApartment;

#[cfg(windows)]
impl Drop for WinRtApartment {
    fn drop(&mut self) {
        unsafe { windows::Win32::System::WinRT::RoUninitialize() };
    }
}

#[cfg(target_os = "macos")]
fn send(title: &str, body: &str) -> std::io::Result<()> {
    use std::process::Command;

    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        escape_applescript(body),
        escape_applescript(title)
    );
    command_succeeded(Command::new("osascript").args(["-e", &script]))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn send(title: &str, body: &str) -> std::io::Result<()> {
    use std::process::Command;

    command_succeeded(Command::new("notify-send").args([
        "--app-name",
        "OpenCode Quota Checker",
        title,
        body,
    ]))
}

#[cfg(any(target_os = "macos", all(unix, not(target_os = "macos"))))]
fn command_succeeded(command: &mut std::process::Command) -> std::io::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "notification command exited with {status}"
        )))
    }
}

#[cfg(target_os = "macos")]
fn escape_applescript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(windows)]
fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn notification_text_is_xml_escaped() {
        assert_eq!(super::escape_xml("<&\"'>"), "&lt;&amp;&quot;&apos;&gt;");
    }
}
