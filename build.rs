//! Reads the versions of the two bundled apps from their workspaces under `vendor/`, and on
//! Windows embeds the icon and version info into `CraftSuite.exe`.

use std::path::Path;

fn workspace_version(manifest: &Path) -> String {
    println!("cargo:rerun-if-changed={}", manifest.display());
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return "?".into();
    };
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package && let Some(value) = line.strip_prefix("version") {
            if let Some(v) = value.trim_start().strip_prefix('=') {
                return v.trim().trim_matches('"').to_string();
            }
        }
    }
    "?".into()
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/craftsuite.ico");
    println!("cargo:rustc-env=FILMCRAFT_VERSION={}", workspace_version(Path::new("vendor/filmcraft/Cargo.toml")));
    println!("cargo:rustc-env=EFFECTCRAFT_VERSION={}", workspace_version(Path::new("vendor/effectcraft/Cargo.toml")));

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/craftsuite.ico")
        .set("ProductName", "CraftSuite")
        .set("FileDescription", "CraftSuite: FilmCraft und EffectCraft")
        .set("LegalCopyright", "MIT OR Apache-2.0. FilmCraft und EffectCraft: The FilmCraft / EffectCraft contributors.")
        .set("OriginalFilename", "CraftSuite.exe")
        .set("InternalName", "craftsuite");
    if let Err(e) = res.compile() {
        if std::env::var_os("CRAFTSUITE_REQUIRE_WINRES").is_some() {
            panic!("embedding Windows resources failed: {e}");
        }
        println!("cargo:warning=CraftSuite.exe built without icon/version resources: {e}");
    }
}
