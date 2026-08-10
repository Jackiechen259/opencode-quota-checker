use std::{
    collections::BTreeMap,
    env, fmt, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    str::FromStr,
};

use sha2::{Digest, Sha256};

const PACKAGER_VERSION_KEY: &str = "\"version\": \"";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(usage());
    };
    let root = workspace_root();

    match command.as_str() {
        "release" => {
            let bump = args.next().ok_or_else(usage)?;
            let push = args.any(|arg| arg == "--push");
            release(&root, &bump, push)
        }
        "verify-version" => {
            let tag = args.next();
            verify_version(&root, tag.as_deref())
        }
        "update-manifest" => {
            let tag = args.next().ok_or_else(usage)?;
            let dir = args.next().ok_or_else(usage)?;
            update_manifest(&root, &tag, &dir)
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: cargo xtask release <patch|minor|major|VERSION> [--push]\n\
     or:    cargo xtask verify-version [vVERSION]\n\
     or:    cargo xtask update-manifest <vVERSION> <asset-directory>"
        .to_owned()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be a workspace member")
        .to_owned()
}

fn release(root: &Path, bump: &str, push: bool) -> Result<(), String> {
    ensure_clean(root)?;

    let current = workspace_version(root)?;
    let next = match bump {
        "patch" => current.bump_patch(),
        "minor" => current.bump_minor(),
        "major" => current.bump_major(),
        explicit => Version::from_str(explicit)?,
    };

    if next == current {
        return Err(format!("version is already {current}"));
    }

    update_workspace_version(root, &next)?;
    update_packager_version(root, &next)?;

    run_command(root, "cargo", &["fmt", "--all"])?;
    run_command(root, "cargo", &["check", "--workspace"])?;
    run_command(
        root,
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_command(root, "cargo", &["test", "--workspace"])?;
    verify_version(root, Some(&format!("v{next}")))?;

    run_command(
        root,
        "git",
        &[
            "add",
            "Cargo.toml",
            "Cargo.lock",
            "crates/opencode-desktop/packager.json",
        ],
    )?;
    run_command(
        root,
        "git",
        &["commit", "-m", &format!("chore(release): v{next}")],
    )?;
    run_command(
        root,
        "git",
        &["tag", "-a", &format!("v{next}"), "-m", &format!("v{next}")],
    )?;

    if push {
        run_command(root, "git", &["push", "origin", "HEAD"])?;
        run_command(root, "git", &["push", "origin", &format!("v{next}")])?;
    }

    println!("prepared release v{next}");
    Ok(())
}

fn verify_version(root: &Path, tag: Option<&str>) -> Result<(), String> {
    let workspace = workspace_version(root)?;
    let packager = packager_version(root)?;

    if workspace != packager {
        return Err(format!(
            "version mismatch: workspace={workspace}, packager={packager}"
        ));
    }

    if let Some(tag) = tag {
        let tagged = Version::from_str(tag.strip_prefix('v').unwrap_or(tag))?;
        if workspace != tagged {
            return Err(format!(
                "tag v{tagged} does not match workspace version {workspace}"
            ));
        }
    }

    println!("version {workspace} is consistent");
    Ok(())
}

/// Generates `SHA256SUMS` and `update.json` for a release tag from the
/// package assets in `asset_directory`. Fails when any platform asset that
/// the updater relies on is missing, so a release can never point at files
/// that do not exist.
fn update_manifest(root: &Path, tag: &str, asset_directory: &str) -> Result<(), String> {
    let repository = workspace_repository(root)?;
    update_manifest_at(&repository, tag, &root.join(asset_directory))
}

fn update_manifest_at(repository: &str, tag: &str, asset_dir: &Path) -> Result<(), String> {
    Version::from_str(tag.strip_prefix('v').unwrap_or(tag))?;
    if !asset_dir.is_dir() {
        return Err(format!(
            "asset directory does not exist: {}",
            asset_dir.display()
        ));
    }

    let mut platforms = BTreeMap::new();
    for spec in REQUIRED_ASSETS {
        let path = asset_dir.join(spec.filename);
        if !path.is_file() {
            return Err(format!("missing required asset: {}", spec.filename));
        }
        let digest = sha256_of(&path)?;
        platforms.insert(
            spec.key.to_owned(),
            UpdatePlatform {
                kind: spec.kind,
                url: format!("{repository}/releases/download/{tag}/{}", spec.filename),
                sha256: digest,
            },
        );
    }

    let sums_path = asset_dir.join("SHA256SUMS");
    fs::write(&sums_path, checksums_for(asset_dir)?)
        .map_err(|error| format!("failed to write {}: {error}", sums_path.display()))?;

    let manifest = UpdateManifest {
        schema: 1,
        version: tag.strip_prefix('v').unwrap_or(tag).to_owned(),
        tag: tag.to_owned(),
        prerelease: tag.contains('-'),
        release_notes_url: format!("{repository}/releases/tag/{tag}"),
        platforms,
    };
    let manifest_path = asset_dir.join("update.json");
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("failed to serialize update.json: {error}"))?;
    fs::write(&manifest_path, format!("{json}\n"))
        .map_err(|error| format!("failed to write {}: {error}", manifest_path.display()))?;

    for path in sorted_files(asset_dir)? {
        let name = file_name(&path);
        if name != "SHA256SUMS"
            && name != "update.json"
            && !REQUIRED_ASSETS.iter().any(|spec| spec.filename == name)
        {
            println!("warning: unexpected file in release: {name}");
        }
    }

    println!(
        "generated update.json and SHA256SUMS for {tag} in {}",
        asset_dir.display()
    );
    Ok(())
}

/// `{sha256}  {filename}` lines over every file in the directory, sorted by name.
fn checksums_for(dir: &Path) -> Result<String, String> {
    let mut output = String::new();
    for path in sorted_files(dir)? {
        output.push_str(&format!("{}  {}\n", sha256_of(&path)?, file_name(&path)));
    }
    Ok(output)
}

fn sorted_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = fs::read_dir(dir)
        .map_err(|error| format!("failed to read {}: {error}", dir.display()))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn sha256_of(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Ok(hex_encode(&Sha256::digest(&bytes)))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Release asset contract shared with the desktop updater. Only these
/// platforms are built and published; macOS Intel is intentionally absent.
struct AssetSpec {
    key: &'static str,
    kind: &'static str,
    filename: &'static str,
}

const REQUIRED_ASSETS: [AssetSpec; 4] = [
    AssetSpec {
        key: "windows-x86_64",
        kind: "nsis",
        filename: "opencode-quota-checker-windows-x86_64.exe",
    },
    AssetSpec {
        key: "linux-x86_64-appimage",
        kind: "appimage",
        filename: "opencode-quota-checker-linux-x86_64.AppImage",
    },
    AssetSpec {
        key: "linux-x86_64-deb",
        kind: "deb",
        filename: "opencode-quota-checker-linux-x86_64.deb",
    },
    AssetSpec {
        key: "macos-aarch64",
        kind: "dmg",
        filename: "opencode-quota-checker-macos-aarch64.dmg",
    },
];

#[derive(serde::Serialize)]
struct UpdateManifest {
    schema: u32,
    version: String,
    tag: String,
    prerelease: bool,
    release_notes_url: String,
    platforms: BTreeMap<String, UpdatePlatform>,
}

#[derive(serde::Serialize)]
struct UpdatePlatform {
    #[serde(rename = "type")]
    kind: &'static str,
    url: String,
    sha256: String,
}

fn ensure_clean(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to run git status: {error}"))?;
    if !output.status.success() {
        return Err("git status failed".to_owned());
    }
    if !output.stdout.is_empty() {
        return Err("working tree must be clean before preparing a release".to_owned());
    }
    Ok(())
}

fn workspace_version(root: &Path) -> Result<Version, String> {
    let manifest = read(root.join("Cargo.toml"))?;
    let section = manifest
        .split_once("[workspace.package]")
        .ok_or("missing [workspace.package]")?
        .1;
    let line = section
        .lines()
        .find(|line| line.trim_start().starts_with("version = "))
        .ok_or("missing workspace version")?;
    Version::from_str(quoted_value(line)?)
}

fn workspace_repository(root: &Path) -> Result<String, String> {
    let manifest = read(root.join("Cargo.toml"))?;
    let section = manifest
        .split_once("[workspace.package]")
        .ok_or("missing [workspace.package]")?
        .1;
    let line = section
        .lines()
        .find(|line| line.trim_start().starts_with("repository = "))
        .ok_or("missing workspace repository")?;
    Ok(quoted_value(line)?.to_owned())
}

fn packager_version(root: &Path) -> Result<Version, String> {
    let config = read(root.join("crates/opencode-desktop/packager.json"))?;
    let value = config
        .find(PACKAGER_VERSION_KEY)
        .map(|start| &config[start + PACKAGER_VERSION_KEY.len()..])
        .ok_or("missing packager version")?;
    let end = value.find('"').ok_or("unterminated packager version")?;
    Version::from_str(&value[..end])
}

fn update_workspace_version(root: &Path, version: &Version) -> Result<(), String> {
    let path = root.join("Cargo.toml");
    let manifest = read(&path)?;
    let marker = "[workspace.package]";
    let start = manifest.find(marker).ok_or("missing [workspace.package]")?;
    let relative = manifest[start..]
        .find("version = \"")
        .ok_or("missing workspace version")?;
    let value_start = start + relative + "version = \"".len();
    replace_quoted_value(&path, manifest, value_start, version)
}

fn update_packager_version(root: &Path, version: &Version) -> Result<(), String> {
    let path = root.join("crates/opencode-desktop/packager.json");
    let config = read(&path)?;
    let start = config
        .find(PACKAGER_VERSION_KEY)
        .ok_or("missing packager version")?
        + PACKAGER_VERSION_KEY.len();
    replace_quoted_value(&path, config, start, version)
}

fn replace_quoted_value(
    path: &Path,
    mut contents: String,
    value_start: usize,
    version: &Version,
) -> Result<(), String> {
    let value_end = contents[value_start..]
        .find('"')
        .map(|offset| value_start + offset)
        .ok_or("unterminated version string")?;
    contents.replace_range(value_start..value_end, &version.to_string());
    fs::write(path, contents)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn quoted_value(line: &str) -> Result<&str, String> {
    let start = line.find('"').ok_or("missing opening quote")? + 1;
    let end = line[start..]
        .find('"')
        .map(|offset| start + offset)
        .ok_or("missing closing quote")?;
    Ok(&line[start..end])
}

fn read(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn run_command(root: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    println!("> {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Option<String>,
}

impl Version {
    fn bump_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            prerelease: None,
        }
    }

    fn bump_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
            prerelease: None,
        }
    }

    fn bump_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            prerelease: None,
        }
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (core, prerelease) = match input.split_once('-') {
            Some((core, suffix)) if valid_prerelease(suffix) => (core, Some(suffix.to_owned())),
            Some(_) => return Err(format!("invalid prerelease version: {input}")),
            None => (input, None),
        };
        let mut parts = core.split('.');
        let major = parse_part(parts.next(), input)?;
        let minor = parse_part(parts.next(), input)?;
        let patch = parse_part(parts.next(), input)?;
        if parts.next().is_some() {
            return Err(format!("invalid version: {input}"));
        }
        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(prerelease) = &self.prerelease {
            write!(formatter, "-{prerelease}")?;
        }
        Ok(())
    }
}

fn parse_part(part: Option<&str>, input: &str) -> Result<u64, String> {
    part.ok_or_else(|| format!("invalid version: {input}"))?
        .parse()
        .map_err(|_| format!("invalid version: {input}"))
}

fn valid_prerelease(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn workspace_and_packager_versions_stay_in_sync() {
        let root = workspace_root();
        assert_eq!(
            workspace_version(&root).expect("workspace version"),
            packager_version(&root).expect("packager version"),
            "cargo xtask release keeps these in sync; run it after bumping the version"
        );
    }

    #[test]
    fn packager_config_matches_the_release_contract() {
        let root = workspace_root();
        let raw = fs::read_to_string(root.join("crates/opencode-desktop/packager.json"))
            .expect("packager.json is readable");
        let config: serde_json::Value =
            serde_json::from_str(&raw).expect("packager.json is valid JSON");

        assert_eq!(config["productName"], "OpenCode Quota Checker");
        assert_eq!(
            config["identifier"], "io.github.jackiechen259.opencode-quota-checker",
            "the application identifier must never change between releases"
        );
        assert_eq!(config["nsis"]["installMode"], "currentUser");
        let languages = config["nsis"]["languages"]
            .as_array()
            .expect("languages is an array");
        let names = languages
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"English"));
        assert!(names.contains(&"SimpChinese"));
        let formats = config["formats"]
            .as_array()
            .expect("formats is an array")
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>();
        assert!(
            formats.contains(&"default") || formats.contains(&"nsis"),
            "the Windows package must be NSIS (CI passes --formats nsis explicitly)"
        );
        let main = config["binaries"]
            .as_array()
            .expect("binaries is an array")
            .iter()
            .find(|binary| binary["main"] == true)
            .expect("a main binary is declared");
        assert_eq!(main["path"], "opencode-quota-checker");
    }

    #[test]
    fn parses_stable_and_prerelease_versions() {
        assert_eq!(
            Version::from_str("1.2.3").expect("stable").to_string(),
            "1.2.3"
        );
        assert_eq!(
            Version::from_str("1.2.3-rc.1")
                .expect("prerelease")
                .to_string(),
            "1.2.3-rc.1"
        );
    }

    #[test]
    fn bumps_versions_and_drops_prerelease() {
        let version = Version::from_str("1.2.3-rc.1").expect("version");
        assert_eq!(version.bump_patch().to_string(), "1.2.4");
        assert_eq!(version.bump_minor().to_string(), "1.3.0");
        assert_eq!(version.bump_major().to_string(), "2.0.0");
    }

    #[test]
    fn generates_manifest_for_all_required_assets() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let dir = directory.path();
        for spec in REQUIRED_ASSETS {
            fs::write(dir.join(spec.filename), b"package").expect("asset is written");
        }

        update_manifest_at(
            "https://github.com/example/opencode-quota-checker",
            "v0.2.0",
            dir,
        )
        .expect("manifest generates");

        let raw = fs::read_to_string(dir.join("update.json")).expect("update.json is written");
        let manifest: serde_json::Value =
            serde_json::from_str(&raw).expect("update.json is valid JSON");
        assert_eq!(manifest["schema"], 1);
        assert_eq!(manifest["version"], "0.2.0");
        assert_eq!(manifest["tag"], "v0.2.0");
        assert_eq!(manifest["prerelease"], false);
        assert_eq!(
            manifest["release_notes_url"],
            "https://github.com/example/opencode-quota-checker/releases/tag/v0.2.0"
        );

        let platforms = manifest["platforms"]
            .as_object()
            .expect("platforms is an object");
        assert_eq!(platforms.len(), 4);
        assert!(!platforms.contains_key("macos-x86_64"));
        let windows = platforms["windows-x86_64"]
            .as_object()
            .expect("platform entry");
        assert_eq!(windows["type"], "nsis");
        assert_eq!(
            windows["url"],
            "https://github.com/example/opencode-quota-checker/releases/download/v0.2.0/opencode-quota-checker-windows-x86_64.exe"
        );
        assert_eq!(
            windows["sha256"],
            "bc4a71180870f7945155fbb02f4b0a2e3faa2a62d6d31b7039013055ed19869a"
        );
    }

    #[test]
    fn writes_sha256sums_for_every_asset() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let dir = directory.path();
        fs::write(dir.join(REQUIRED_ASSETS[0].filename), b"package").expect("asset is written");
        for spec in &REQUIRED_ASSETS[1..] {
            fs::write(dir.join(spec.filename), b"package").expect("asset is written");
        }

        update_manifest_at(
            "https://github.com/example/opencode-quota-checker",
            "v0.2.0",
            dir,
        )
        .expect("manifest generates");

        let sums = fs::read_to_string(dir.join("SHA256SUMS")).expect("SHA256SUMS is written");
        let lines = sums.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 4);
        assert!(lines.iter().all(|line| {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            parts.len() == 2
                && parts[0].len() == 64
                && parts[0]
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
                && REQUIRED_ASSETS.iter().any(|spec| spec.filename == parts[1])
        }));
    }

    #[test]
    fn fails_when_a_required_asset_is_missing() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let dir = directory.path();
        fs::write(dir.join(REQUIRED_ASSETS[0].filename), b"package").expect("asset is written");

        let error = update_manifest_at(
            "https://github.com/example/opencode-quota-checker",
            "v0.2.0",
            dir,
        )
        .expect_err("missing assets must fail");
        assert!(error.contains("missing required asset"));
    }

    #[test]
    fn marks_prerelease_tags_and_rejects_invalid_versions() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let dir = directory.path();
        for spec in REQUIRED_ASSETS {
            fs::write(dir.join(spec.filename), b"package").expect("asset is written");
        }

        update_manifest_at(
            "https://github.com/example/opencode-quota-checker",
            "v1.0.0-rc.1",
            dir,
        )
        .expect("prerelease manifest generates");
        let raw = fs::read_to_string(dir.join("update.json")).expect("update.json is written");
        let manifest: serde_json::Value =
            serde_json::from_str(&raw).expect("update.json is valid JSON");
        assert_eq!(manifest["prerelease"], true);

        assert!(update_manifest_at(
            "https://github.com/example/opencode-quota-checker",
            "not-a-version",
            dir,
        )
        .is_err());
    }
}
