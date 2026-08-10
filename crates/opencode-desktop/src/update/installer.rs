//! Launching the verified package through the platform installer.
//!
//! The first version never installs unattended: Windows launches the NSIS
//! installer, macOS opens the DMG, Linux replaces a running AppImage in place
//! (or opens the package), and debug builds refuse to install at all.

use crate::state::UiError;
use crate::update::{download::VerifiedPackage, manifest::PackageType, UpdateError};
use std::path::Path;
use std::process::Command;

/// Installs the verified package, or refuses for debug/unsupported builds.
///
/// Returns `Ok(true)` when an installer or replacement process was launched
/// and the application should exit, or `Ok(false)` when the package was only
/// opened for the user and the application keeps running (macOS DMG, Linux
/// deb). The package is never executed before its checksum was verified.
pub fn install_update(package: &VerifiedPackage) -> Result<bool, UiError> {
    if cfg!(debug_assertions) {
        return Err(UpdateError::Install("debug builds cannot install updates".to_owned()).into());
    }
    install_for_platform(&package.path, package.kind)
        .map_err(|error| UpdateError::Install(error).into())
}

#[cfg(target_os = "windows")]
fn install_for_platform(path: &Path, kind: PackageType) -> Result<bool, String> {
    match kind {
        PackageType::Nsis => {
            // `/UPDATE` signals the NSIS installer that this app is exiting on
            // its own right now, so its custom pre-install logic waits for the
            // process to terminate instead of prompting the user to close it.
            // The flag is unknown to stock NSIS command lines and is ignored by
            // every other installer, so it is safe to always pass.
            Command::new(path)
                .arg("/UPDATE")
                .spawn()
                .map_err(|error| format!("failed to start installer: {error}"))?;
            Ok(true)
        }
        _ => Err(format!(
            "unsupported package type for this platform: {kind:?}"
        )),
    }
}

#[cfg(target_os = "macos")]
fn install_for_platform(path: &Path, kind: PackageType) -> Result<bool, String> {
    match kind {
        PackageType::Dmg => {
            Command::new("open")
                .arg(path)
                .spawn()
                .map_err(|error| format!("failed to open disk image: {error}"))?;
            Ok(false)
        }
        _ => Err(format!(
            "unsupported package type for this platform: {kind:?}"
        )),
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn install_for_platform(path: &Path, kind: PackageType) -> Result<bool, String> {
    match kind {
        PackageType::AppImage => {
            if let Some(appimage) = std::env::var_os("APPIMAGE") {
                match replace_appimage(path, Path::new(&appimage)) {
                    Ok(exit) => return Ok(exit),
                    Err(error) => tracing::warn!(
                        %error,
                        "in-place AppImage update unavailable; launching the package instead"
                    ),
                }
            }
            run_appimage(path)
        }
        PackageType::Deb => {
            Command::new("xdg-open")
                .arg(path)
                .spawn()
                .map_err(|error| format!("failed to open package: {error}"))?;
            Ok(false)
        }
        _ => Err(format!(
            "unsupported package type for this platform: {kind:?}"
        )),
    }
}

#[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
fn install_for_platform(_path: &Path, _kind: PackageType) -> Result<bool, String> {
    Err("automatic updates are not supported on this platform".to_owned())
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn install_for_platform(_path: &Path, _kind: PackageType) -> Result<bool, String> {
    Err("automatic updates are not supported on this platform".to_owned())
}

/// Replaces the currently running AppImage atomically and restarts the app.
///
/// The new image is staged in the AppImage's own directory (so the rename is
/// atomic), fsynced, made executable, and renamed over the running image.
/// POSIX keeps the running process on its original inode, so this does not
/// corrupt the current process; the freshly started process runs the new file.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn replace_appimage(new: &Path, current: &Path) -> Result<bool, String> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let parent = current
        .parent()
        .ok_or_else(|| "cannot resolve the running AppImage directory".to_owned())?;
    let temp = parent.join(format!(
        ".opencode-quota-checker-update-{}.AppImage",
        std::process::id()
    ));
    fs::copy(new, &temp).map_err(|error| format!("cannot stage AppImage: {error}"))?;
    let file =
        fs::File::open(&temp).map_err(|error| format!("cannot open staged AppImage: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync staged AppImage: {error}"))?;
    drop(file);
    fs::set_permissions(&temp, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("cannot make staged AppImage executable: {error}"))?;
    fs::rename(&temp, current).map_err(|error| format!("cannot replace AppImage: {error}"))?;
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot locate the current executable: {error}"))?;
    Command::new(exe)
        .spawn()
        .map_err(|error| format!("cannot restart the application: {error}"))?;
    Ok(true)
}

/// Fallback when no in-place AppImage update is possible: launch the downloaded
/// image directly so the user still reaches the new version.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn run_appimage(path: &Path) -> Result<bool, String> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("cannot make AppImage executable: {error}"))?;
    Command::new(path)
        .spawn()
        .map_err(|error| format!("failed to launch AppImage: {error}"))?;
    Ok(true)
}

/// Opens `url` in the system default browser.
pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
