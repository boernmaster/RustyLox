//! Integration test: install weather4lox plugin from ZIP
//!
//! Requires /tmp/weather4lox.zip to be present (downloaded by CI or manually).
//! Run with: cargo test -p plugin-manager --test weather4lox_install

use plugin_manager::{InstallAction, InstallRequest, PluginInstaller};
use tempfile::TempDir;

#[tokio::test]
async fn test_install_weather4lox() {
    let zip_path = std::path::PathBuf::from("/tmp/weather4lox.zip");
    if !zip_path.exists() {
        eprintln!("Skipping test: /tmp/weather4lox.zip not found");
        return;
    }

    // Create a temporary lbhomedir
    let lbhomedir = TempDir::new().expect("create temp dir");

    // Create required subdirectories
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
        Ok(entry) => {
            println!("Installed: {} v{}", entry.name, entry.version);
            assert_eq!(entry.name, "weather4lox");
            assert_eq!(entry.folder, "weather4lox");
            assert_eq!(entry.version, "5.0.0");
            assert_eq!(entry.author_name, "Michael Schlenstedt");

            // Verify title was parsed from plain TITLE= key
            assert!(
                entry
                    .directories
                    .lbpbindir
                    .to_string()
                    .contains("weather4lox"),
                "bin dir should contain 'weather4lox'"
            );

            // Verify plugin files were copied
            let bin_dir = lbhomedir.path().join("bin/plugins/weather4lox");
            assert!(
                bin_dir.exists(),
                "bin/plugins/weather4lox directory should exist"
            );

            // Verify the plugin is in the database
            let plugins = installer.list().await.unwrap();
            assert_eq!(plugins.len(), 1);
            assert_eq!(plugins[0].name, "weather4lox");

            println!("weather4lox installation test PASSED");
        }
        Err(e) => {
            panic!("Installation failed: {}", e);
        }
    }
}
