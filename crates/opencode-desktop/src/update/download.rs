//! Downloading a platform package, verifying its SHA-256 against the
//! manifest, and atomically landing it in the user cache directory.
//!
//! The download is streamed to a `.partial` file with progress messages, then
//! verified, then renamed to its final name. Any failure removes the partial
//! file; a package is never executed before its checksum matches.

use crate::message::Message;
use crate::state::UiError;
use crate::update::{checker::UpdateInfo, manifest::PackageType, UpdateError};
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};
use iced::Task;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const PROGRESS_CHANNEL_SIZE: usize = 64;

/// Hosts the updater trusts for package downloads.
const TRUSTED_URL_PREFIXES: [&str; 2] = [
    "https://github.com/",
    "https://objects.githubusercontent.com/",
];

/// A package whose SHA-256 matched the published manifest.
#[derive(Debug, Clone)]
pub struct VerifiedPackage {
    /// Absolute path of the verified package file.
    pub path: PathBuf,
    /// Package format, used to pick the platform installer.
    pub kind: PackageType,
    /// The release version the package installs.
    pub version: String,
}

/// Runs a download as an Iced task, emitting progress messages and ending
/// with an [`Message::UpdateDownloaded`] result.
pub fn download_task(info: UpdateInfo) -> Task<Message> {
    let stream =
        iced::stream::channel(
            PROGRESS_CHANNEL_SIZE,
            async move |mut sender| match download_package(&info, &mut sender).await {
                Ok(package) => {
                    let _ = sender.send(Message::UpdateDownloaded(Ok(package))).await;
                }
                Err(error) => {
                    let _ = sender.send(Message::UpdateDownloaded(Err(error))).await;
                }
            },
        );
    Task::run(stream, std::convert::identity)
}

/// Streams the platform package from the manifest, verifies it, and returns
/// the verified file. Progress is reported on `sender`.
pub async fn download_package(
    info: &UpdateInfo,
    sender: &mut mpsc::Sender<Message>,
) -> Result<VerifiedPackage, UiError> {
    validate_asset_url(&info.platform.url)?;
    let directory = update_directory()?;
    fs::create_dir_all(&directory).map_err(|error| {
        UiError::from(UpdateError::Download(format!(
            "cannot create update directory: {error}"
        )))
    })?;
    let filename = asset_file_name(&info.platform.url)?;
    let partial = directory.join(format!("{filename}.partial"));
    let final_path = directory.join(&filename);

    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(DOWNLOAD_TIMEOUT)
        .user_agent(format!(
            "opencode-quota-checker/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|error| UpdateError::Request(error.to_string()))?;
    let response = client
        .get(&info.platform.url)
        .send()
        .await
        .map_err(|error| UpdateError::Request(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(UiError::from(UpdateError::Download(format!(
            "HTTP {status}"
        ))));
    }
    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = File::create(&partial)
        .map_err(|error| UpdateError::Download(format!("cannot create download: {error}")))?;
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| UpdateError::Download(error.to_string()))?;
        file.write_all(&chunk)
            .map_err(|error| UpdateError::Download(format!("cannot write download: {error}")))?;
        downloaded += chunk.len() as u64;
        let _ = sender
            .send(Message::UpdateDownloadProgress { downloaded, total })
            .await;
    }
    file.sync_all()
        .map_err(|error| UpdateError::Download(format!("cannot sync download: {error}")))?;
    drop(file);

    verify_and_delete_on_mismatch(&partial, &info.platform.sha256)?;
    fs::rename(&partial, &final_path).map_err(|error| {
        UiError::from(UpdateError::Download(format!(
            "cannot finalize download: {error}"
        )))
    })?;

    Ok(VerifiedPackage {
        path: final_path,
        kind: info.platform.kind,
        version: info.version.to_string(),
    })
}

/// Validates the asset URL against the trusted GitHub release origins.
fn validate_asset_url(url: &str) -> Result<(), UpdateError> {
    if TRUSTED_URL_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix))
    {
        Ok(())
    } else {
        Err(UpdateError::UnsafeUrl)
    }
}

/// Derives the package file name from the trusted asset URL.
fn asset_file_name(url: &str) -> Result<String, UpdateError> {
    url.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| UpdateError::Download(format!("cannot derive asset filename from {url}")))
}

/// Verifies `file` against the expected hex digest, deleting it on mismatch.
fn verify_and_delete_on_mismatch(file: &Path, expected_hex: &str) -> Result<(), UpdateError> {
    let bytes = fs::read(file)
        .map_err(|error| UpdateError::Download(format!("cannot read download: {error}")))?;
    let actual = hex_encode(&Sha256::digest(&bytes));
    if !checksum_matches(&actual, expected_hex) {
        let _ = fs::remove_file(file);
        return Err(UpdateError::Verification);
    }
    Ok(())
}

/// Lowercase-hex SHA-256 of the file bytes.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Constant-time comparison of two lowercase-hex digests.
fn checksum_matches(actual: &str, expected: &str) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in actual.bytes().zip(expected.bytes()) {
        diff |= left ^ right;
    }
    diff == 0
}

/// Platform-standard user cache directory for downloaded updates.
fn update_directory() -> Result<PathBuf, UpdateError> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| UpdateError::Download("user cache directory is unavailable".to_owned()))?;
    Ok(base
        .cache_dir()
        .join("opencode-quota-checker")
        .join("update"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encoding_is_lowercase() {
        assert_eq!(hex_encode(&[0xDE, 0xAD, 0xBE, 0xEF]), "deadbeef");
        assert_eq!(hex_encode(b""), "");
    }

    #[test]
    fn checksums_match_only_when_equal() {
        let digest = "bc4a71180870f7945155fbb02f4b0a2e3faa2a62d6d31b7039013055ed19869a";
        assert!(checksum_matches(digest, digest));
        assert!(!checksum_matches(digest, &format!("a{digest}")));
        assert!(!checksum_matches(digest, &digest.to_uppercase()));
    }

    #[test]
    fn trusted_urls_are_accepted_and_untrusted_rejected() {
        assert!(validate_asset_url(
            "https://github.com/Jackiechen259/opencode-quota-checker/releases/download/v0.2.0/opencode-quota-checker-windows-x86_64.exe"
        )
        .is_ok());
        assert!(
            validate_asset_url("https://objects.githubusercontent.com/some-bucket/asset.exe")
                .is_ok()
        );
        assert!(validate_asset_url("https://evil.example.com/asset.exe").is_err());
        assert!(validate_asset_url("http://github.com/asset.exe").is_err());
    }

    #[test]
    fn asset_filename_is_derived_from_the_url() {
        assert_eq!(
            asset_file_name("https://github.com/x/y/releases/download/v0.2.0/opencode-quota-checker-macos-aarch64.dmg")
                .expect("filename"),
            "opencode-quota-checker-macos-aarch64.dmg"
        );
        assert!(asset_file_name("https://github.com/").is_err());
    }

    #[test]
    fn correct_checksum_verifies() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let file = directory.path().join("package.exe");
        let mut handle = File::create(&file).expect("file is created");
        handle.write_all(b"package").expect("file is written");
        handle.sync_all().expect("file is synced");

        let expected = "bc4a71180870f7945155fbb02f4b0a2e3faa2a62d6d31b7039013055ed19869a";
        verify_and_delete_on_mismatch(&file, expected).expect("checksum matches");
        assert!(file.exists());
    }

    #[test]
    fn incorrect_checksum_fails_and_deletes_the_file() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let file = directory.path().join("package.exe");
        fs::write(&file, b"package").expect("file is written");

        let expected = "0".repeat(64);
        assert!(matches!(
            verify_and_delete_on_mismatch(&file, &expected),
            Err(UpdateError::Verification)
        ));
        assert!(!file.exists(), "mismatched download must be deleted");
    }

    #[test]
    fn truncated_download_fails_and_deletes_the_file() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let file = directory.path().join("package.exe");
        fs::write(&file, b"pa").expect("truncated file is written");

        let expected = "bc4a71180870f7945155fbb02f4b0a2e3faa2a62d6d31b7039013055ed19869a";
        assert!(verify_and_delete_on_mismatch(&file, expected).is_err());
        assert!(!file.exists(), "truncated download must be deleted");
    }
}
