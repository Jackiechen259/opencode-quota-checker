//! Embeds the application icon into the Windows executable.
//!
//! The ICO resource makes Explorer, Start Menu shortcuts, the taskbar and
//! Alt+Tab identify `opencode-quota-checker.exe` by the project icon instead
//! of the generic executable icon. It is independent of the runtime window
//! and tray icons configured in `src/platform/icon.rs`.

use std::io;

fn main() -> io::Result<()> {
    // `cfg!(windows)` in a build script reflects the *host* platform, so rely
    // on cargo's target environment variable instead. The `#[cfg(windows)]`
    // guard additionally keeps this script compiling when a non-Windows host
    // builds a Windows target (where `winresource` is not a dependency).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=../../assets/icons/icon.ico");

        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icons/icon.ico");
        res.compile()?;
    }

    Ok(())
}
