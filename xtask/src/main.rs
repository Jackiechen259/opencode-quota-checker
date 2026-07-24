use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    str::FromStr,
};

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
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: cargo xtask release <patch|minor|major|VERSION> [--push]\n\
     or:    cargo xtask verify-version [vVERSION]"
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
            "crates/volc-desktop/packager.json",
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

fn packager_version(root: &Path) -> Result<Version, String> {
    let config = read(root.join("crates/volc-desktop/packager.json"))?;
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
    let path = root.join("crates/volc-desktop/packager.json");
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
    use super::Version;
    use std::str::FromStr;

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
}
