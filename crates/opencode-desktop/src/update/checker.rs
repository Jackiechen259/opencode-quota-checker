//! Fetching and evaluating the update manifest against the running version.
//!
//! Everything that decides whether an update exists is a pure function
//! (`decide_update`) so it can be tested without the network.

use crate::state::UiError;
use crate::update::{
    manifest::{UpdateManifest, UpdatePlatform},
    UpdateError,
};
use semver::Version;
use std::time::Duration;

/// The stable channel manifest URL. `latest` automatically resolves to the
/// newest non-prerelease release, so no GitHub API or token is required.
pub const UPDATE_MANIFEST_URL: &str =
    "https://github.com/Jackiechen259/opencode-quota-checker/releases/latest/download/update.json";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(15);

/// A platform that has a published update package. Keys match the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateTarget {
    /// Windows x86-64, installed via the NSIS installer.
    WindowsX86_64,
    /// macOS Apple Silicon, delivered as a DMG.
    MacOsAarch64,
    /// Linux x86-64 running from an AppImage.
    LinuxX86_64AppImage,
    /// Linux x86-64 installed from a Debian package.
    LinuxX86_64Deb,
}

impl UpdateTarget {
    /// The manifest platform key for this target.
    pub const fn manifest_key(self) -> &'static str {
        match self {
            Self::WindowsX86_64 => "windows-x86_64",
            Self::MacOsAarch64 => "macos-aarch64",
            Self::LinuxX86_64AppImage => "linux-x86_64-appimage",
            Self::LinuxX86_64Deb => "linux-x86_64-deb",
        }
    }
}

/// Detects the running platform's update target.
///
/// macOS Intel and every other unsupported architecture return `None`; they
/// are never offered a package from another architecture.
pub fn current_update_target() -> Option<UpdateTarget> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some(UpdateTarget::WindowsX86_64),
        ("macos", "aarch64") => Some(UpdateTarget::MacOsAarch64),
        ("linux", "x86_64") => Some(if std::env::var_os("APPIMAGE").is_some() {
            UpdateTarget::LinuxX86_64AppImage
        } else {
            UpdateTarget::LinuxX86_64Deb
        }),
        _ => None,
    }
}

/// A concrete update for the current platform.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// The published remote version.
    pub version: Version,
    /// The full release tag.
    pub tag: String,
    /// Browser URL for release notes.
    pub release_notes_url: String,
    /// The platform package to download.
    pub platform: UpdatePlatform,
}

/// Fetches the stable manifest and decides whether an update applies.
///
/// Unsupported platforms (including Intel macOS) resolve to an
/// [`UpdateError::UnsupportedPlatform`] so the settings page can explain that
/// no package exists; prereleases resolve to `None`. Failures never block the
/// rest of the application.
pub async fn check_for_update() -> Result<Option<UpdateInfo>, UiError> {
    let target =
        current_update_target().ok_or_else(|| UiError::from(UpdateError::UnsupportedPlatform))?;
    let manifest = fetch_manifest().await?;
    Ok(decide_update(&local_version(), target, &manifest))
}

/// Evaluates whether a manifest offers a newer stable release for `target`.
///
/// Returns `None` when the manifest is a prerelease, does not publish the
/// current platform, or offers a version that is not strictly newer.
fn decide_update(
    local: &Version,
    target: UpdateTarget,
    manifest: &UpdateManifest,
) -> Option<UpdateInfo> {
    if manifest.prerelease {
        return None;
    }
    let platform = manifest.platforms.get(target.manifest_key())?;
    let remote = Version::parse(&manifest.version).ok()?;
    if remote <= *local {
        return None;
    }
    Some(UpdateInfo {
        version: remote,
        tag: manifest.tag.clone(),
        release_notes_url: manifest.release_notes_url.clone(),
        platform: platform.clone(),
    })
}

fn local_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("the packaged version is always valid semver")
}

async fn fetch_manifest() -> Result<UpdateManifest, UpdateError> {
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(MANIFEST_TIMEOUT)
        .user_agent(format!(
            "opencode-quota-checker/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|error| UpdateError::Request(error.to_string()))?;
    let response = client
        .get(UPDATE_MANIFEST_URL)
        .send()
        .await
        .map_err(|error| UpdateError::Request(error.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| UpdateError::Request(error.to_string()))?;
    if !status.is_success() {
        return Err(UpdateError::Request(format!("HTTP {status}")));
    }
    UpdateManifest::parse(&body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::manifest::PackageType;
    use std::collections::HashMap;

    fn version(value: &str) -> Version {
        Version::parse(value).expect("test version is valid")
    }

    fn platform() -> UpdatePlatform {
        UpdatePlatform {
            kind: PackageType::Nsis,
            url: "https://github.com/Jackiechen259/opencode-quota-checker/releases/download/v0.2.0/opencode-quota-checker-windows-x86_64.exe".to_owned(),
            sha256: "abc".to_owned(),
        }
    }

    fn manifest(version: &str, prerelease: bool) -> UpdateManifest {
        let mut platforms = HashMap::new();
        platforms.insert("windows-x86_64".to_owned(), platform());
        UpdateManifest {
            schema: 1,
            version: version.to_owned(),
            tag: format!("v{version}"),
            prerelease,
            release_notes_url:
                "https://github.com/Jackiechen259/opencode-quota-checker/releases/tag/v0.2.0"
                    .to_owned(),
            platforms,
        }
    }

    #[test]
    fn newer_version_is_an_update() {
        let update = decide_update(
            &version("0.1.0"),
            UpdateTarget::WindowsX86_64,
            &manifest("0.2.0", false),
        )
        .expect("newer version is available");
        assert_eq!(update.version, version("0.2.0"));
    }

    #[test]
    fn same_version_is_not_an_update() {
        assert!(decide_update(
            &version("0.2.0"),
            UpdateTarget::WindowsX86_64,
            &manifest("0.2.0", false)
        )
        .is_none());
    }

    #[test]
    fn older_version_is_not_an_update() {
        assert!(decide_update(
            &version("0.3.0"),
            UpdateTarget::WindowsX86_64,
            &manifest("0.2.0", false)
        )
        .is_none());
    }

    #[test]
    fn major_upgrade_is_an_update() {
        assert!(decide_update(
            &version("0.2.0"),
            UpdateTarget::WindowsX86_64,
            &manifest("1.0.0", false)
        )
        .is_some());
    }

    #[test]
    fn prereleases_are_ignored_on_the_stable_channel() {
        assert!(decide_update(
            &version("0.1.0"),
            UpdateTarget::WindowsX86_64,
            &manifest("0.2.0-rc.1", true)
        )
        .is_none());
    }

    #[test]
    fn unsupported_platform_gets_no_update() {
        let update = decide_update(
            &version("0.1.0"),
            UpdateTarget::LinuxX86_64Deb,
            &manifest("0.2.0", false),
        );
        assert!(update.is_none());
    }

    #[test]
    fn target_manifest_keys_match_the_published_contract() {
        assert_eq!(UpdateTarget::WindowsX86_64.manifest_key(), "windows-x86_64");
        assert_eq!(UpdateTarget::MacOsAarch64.manifest_key(), "macos-aarch64");
        assert_eq!(
            UpdateTarget::LinuxX86_64AppImage.manifest_key(),
            "linux-x86_64-appimage"
        );
        assert_eq!(
            UpdateTarget::LinuxX86_64Deb.manifest_key(),
            "linux-x86_64-deb"
        );
    }
}
