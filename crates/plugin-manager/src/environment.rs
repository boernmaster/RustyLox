//! Environment variable setup for plugin SDK compatibility

use crate::database::PluginEntry;
use std::collections::HashMap;
use std::path::Path;

/// Build environment variables for plugin execution
/// These match the original LoxBerry SDK environment
pub fn build_plugin_env(plugin: &PluginEntry, lbhomedir: &Path) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // Core paths
    env.insert("LBHOMEDIR".to_string(), lbhomedir.display().to_string());
    env.insert("LBPPLUGINDIR".to_string(), plugin.folder.clone());

    // Plugin-specific paths
    let plugin_folder = &plugin.folder;

    env.insert(
        "LBPHTMLDIR".to_string(),
        format!(
            "{}/webfrontend/html/plugins/{}",
            lbhomedir.display(),
            plugin_folder
        ),
    );

    env.insert(
        "LBPHTMLAUTHDIR".to_string(),
        format!(
            "{}/webfrontend/htmlauth/plugins/{}",
            lbhomedir.display(),
            plugin_folder
        ),
    );

    env.insert(
        "LBPTEMPLATEDIR".to_string(),
        format!(
            "{}/templates/plugins/{}",
            lbhomedir.display(),
            plugin_folder
        ),
    );

    env.insert(
        "LBPDATADIR".to_string(),
        format!("{}/data/plugins/{}", lbhomedir.display(), plugin_folder),
    );

    env.insert(
        "LBPLOGDIR".to_string(),
        format!("{}/log/plugins/{}", lbhomedir.display(), plugin_folder),
    );

    env.insert(
        "LBPCONFIGDIR".to_string(),
        format!("{}/config/plugins/{}", lbhomedir.display(), plugin_folder),
    );

    env.insert(
        "LBPBINDIR".to_string(),
        format!("{}/bin/plugins/{}", lbhomedir.display(), plugin_folder),
    );

    // System paths
    env.insert(
        "LBSHTMLDIR".to_string(),
        format!("{}/webfrontend/html/system", lbhomedir.display()),
    );

    env.insert(
        "LBSHTMLAUTHDIR".to_string(),
        format!("{}/webfrontend/htmlauth/system", lbhomedir.display()),
    );

    env.insert(
        "LBSTEMPLATEDIR".to_string(),
        format!("{}/templates/system", lbhomedir.display()),
    );

    env.insert(
        "LBSDATADIR".to_string(),
        format!("{}/data/system", lbhomedir.display()),
    );

    env.insert(
        "LBSLOGDIR".to_string(),
        format!("{}/log/system", lbhomedir.display()),
    );

    env.insert(
        "LBSTMPFSLOGDIR".to_string(),
        format!("{}/log/system_tmpfs", lbhomedir.display()),
    );

    env.insert(
        "LBSCONFIGDIR".to_string(),
        format!("{}/config/system", lbhomedir.display()),
    );

    env.insert(
        "LBSSBINDIR".to_string(),
        format!("{}/sbin", lbhomedir.display()),
    );

    env.insert(
        "LBSBINDIR".to_string(),
        format!("{}/bin", lbhomedir.display()),
    );

    // Plugin metadata
    env.insert("LBPPLUGINNAME".to_string(), plugin.name.clone());
    env.insert("LBPAUTHOR".to_string(), plugin.author_name.clone());
    env.insert("LBPVERSION".to_string(), plugin.version.clone());

    // Perl library paths
    env.insert(
        "PERL5LIB".to_string(),
        format!("{}/libs/perllib", lbhomedir.display()),
    );

    env
}

/// Build system environment (for non-plugin scripts)
pub fn build_system_env(lbhomedir: &Path) -> HashMap<String, String> {
    let mut env = HashMap::new();

    env.insert("LBHOMEDIR".to_string(), lbhomedir.display().to_string());

    env.insert(
        "LBSHTMLDIR".to_string(),
        format!("{}/webfrontend/html/system", lbhomedir.display()),
    );

    env.insert(
        "LBSHTMLAUTHDIR".to_string(),
        format!("{}/webfrontend/htmlauth/system", lbhomedir.display()),
    );

    env.insert(
        "LBSTEMPLATEDIR".to_string(),
        format!("{}/templates/system", lbhomedir.display()),
    );

    env.insert(
        "LBSDATADIR".to_string(),
        format!("{}/data/system", lbhomedir.display()),
    );

    env.insert(
        "LBSLOGDIR".to_string(),
        format!("{}/log/system", lbhomedir.display()),
    );

    env.insert(
        "LBSCONFIGDIR".to_string(),
        format!("{}/config/system", lbhomedir.display()),
    );

    env.insert(
        "PERL5LIB".to_string(),
        format!("{}/libs/perllib", lbhomedir.display()),
    );

    env
}
