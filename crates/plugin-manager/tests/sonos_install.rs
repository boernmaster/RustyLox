//! Integration test: install LoxBerry-Sonos plugin from ZIP
//!
//! Requires /tmp/sonos.zip to be present.
//! Run with: cargo test -p plugin-manager --test sonos_install -- --nocapture

use plugin_manager::{InstallAction, InstallRequest, PluginInstaller};
use tempfile::TempDir;

#[tokio::test]
async fn test_install_sonos() {
    let zip_path = std::path::PathBuf::from("/tmp/sonos.zip");
    if !zip_path.exists() {
        eprintln!("Skipping test: /tmp/sonos.zip not found");
        return;
    }

    let lbhomedir = TempDir::new().expect("create temp dir");

    for dir in &[
        "data/system",
        "config/system",
        "log/system",
        "bin/plugins",
        "webfrontend/htmlauth/plugins",
        "webfrontend/html/plugins",
        "templates/plugins",
        "data/plugins",
        "config/plugins",
        "log/plugins",
        "tmp",
    ] {
        tokio::fs::create_dir_all(lbhomedir.path().join(dir))
            .await
            .unwrap_or_default();
    }

    let installer = PluginInstaller::new(lbhomedir.path());

    let result = installer
        .install(InstallRequest {
            zip_path,
            action: InstallAction::Install,
            force: false,
        })
        .await;

    match &result {
        Ok(entry) => println!(
            "Installed: {} v{} (folder: {})",
            entry.name, entry.version, entry.folder
        ),
        Err(e) => println!("Installation failed: {}", e),
    }

    // Print installed files for inspection
    if result.is_ok() {
        let plugins = installer.list().await.unwrap();
        println!(
            "Plugins in DB: {:?}",
            plugins.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
    }
}
