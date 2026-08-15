fn main() {
    tauri_build::build();
    embed_common_controls_manifest_for_tests();
}

/// Test binaries link `muda`/`tauri-runtime-wry` code that imports
/// comctl32 v6 entry points (`SetWindowSubclass`, `TaskDialogIndirect`, …).
/// The packaged app resolves them through tauri-build's application manifest,
/// but the test harness gets no manifest, so the loader binds comctl32 5.82
/// and dies with STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139) before `main` runs.
///
/// Embedding the Common-Controls v6 dependency into every test binary makes
/// `cargo test --workspace` work on Windows.
#[cfg(windows)]
fn embed_common_controls_manifest_for_tests() {
    use std::path::PathBuf;

    let manifest = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set"));
    let path = out_dir.join("common-controls-v6.manifest");
    std::fs::write(&path, manifest).expect("write common-controls manifest");
    // Scoped to test targets so the packaged app binary keeps tauri-build's
    // own manifest untouched. Requires at least one declared `[[test]]`
    // target (tests/updater.rs), otherwise cargo rejects the directive.
    println!(
        "cargo:rustc-link-arg-tests=/MANIFEST:EMBED\ncargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        path.display()
    );
}

#[cfg(not(windows))]
fn embed_common_controls_manifest_for_tests() {}
