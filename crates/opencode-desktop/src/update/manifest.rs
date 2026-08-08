//! Parsing and validation of the `update.json` release manifest published to
//! GitHub Releases by `cargo xtask update-manifest`.

use crate::update::UpdateError;
use serde::Deserialize;
use std::collections::HashMap;

/// The only manifest schema this client understands.
pub const SUPPORTED_SCHEMA: u32 = 1;

/// Package format used by the updater, matching the `type` field of each
/// platform entry in the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
    /// Windows NSIS installer.
    Nsis,
    /// Linux portable application image.
    AppImage,
    /// Linux Debian package.
    Deb,
    /// macOS disk image.
    Dmg,
}

/// One platform's update package inside a manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePlatform {
    /// Package format of the asset.
    #[serde(rename = "type")]
    pub kind: PackageType,
    /// HTTPS download URL of the asset.
    pub url: String,
    /// SHA-256 of the asset in lowercase hex.
    pub sha256: String,
}

/// The full `update.json` document.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateManifest {
    /// Manifest schema version; must equal [`SUPPORTED_SCHEMA`].
    pub schema: u32,
    /// Release version without the `v` prefix, e.g. `"0.2.0"`.
    pub version: String,
    /// Full release tag, e.g. `"v0.2.0"`.
    pub tag: String,
    /// Whether the release is a prerelease that stable users must ignore.
    pub prerelease: bool,
    /// Browser URL for the release notes.
    pub release_notes_url: String,
    /// Per-platform update packages keyed by manifest platform key.
    pub platforms: HashMap<String, UpdatePlatform>,
}

impl UpdateManifest {
    /// Parses and validates a raw `update.json` body.
    pub fn parse(json: &str) -> Result<Self, UpdateError> {
        let manifest: Self = serde_json::from_str(json)
            .map_err(|error| UpdateError::Manifest(format!("cannot parse: {error}")))?;
        if manifest.schema != SUPPORTED_SCHEMA {
            return Err(UpdateError::Manifest(format!(
                "unsupported schema {} (expected {SUPPORTED_SCHEMA})",
                manifest.schema
            )));
        }
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> &'static str {
        r#"{
            "schema": 1,
            "version": "0.2.0",
            "tag": "v0.2.0",
            "prerelease": false,
            "release_notes_url": "https://github.com/Jackiechen259/opencode-quota-checker/releases/tag/v0.2.0",
            "platforms": {
                "windows-x86_64": {
                    "type": "nsis",
                    "url": "https://github.com/Jackiechen259/opencode-quota-checker/releases/download/v0.2.0/opencode-quota-checker-windows-x86_64.exe",
                    "sha256": "abc"
                }
            }
        }"#
    }

    #[test]
    fn parses_a_valid_manifest() {
        let manifest = UpdateManifest::parse(sample_manifest()).expect("manifest parses");
        assert_eq!(manifest.schema, 1);
        assert_eq!(manifest.version, "0.2.0");
        assert!(!manifest.prerelease);
        let platform = manifest
            .platforms
            .get("windows-x86_64")
            .expect("windows platform present");
        assert_eq!(platform.kind, PackageType::Nsis);
        assert_eq!(platform.sha256, "abc");
    }

    #[test]
    fn rejects_invalid_json() {
        let error = UpdateManifest::parse("{ not json").expect_err("invalid JSON fails");
        assert!(matches!(error, UpdateError::Manifest(_)));
    }

    #[test]
    fn rejects_an_unsupported_schema() {
        let json = sample_manifest().replace("\"schema\": 1", "\"schema\": 2");
        let error = UpdateManifest::parse(&json).expect_err("unsupported schema fails");
        assert!(error.to_string().contains("unsupported schema"));
    }
}
