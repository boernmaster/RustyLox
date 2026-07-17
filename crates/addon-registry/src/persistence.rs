use std::collections::HashMap;
use std::path::Path;

use rustylox_core::{Error, Result};
use tokio::fs;

use crate::model::AddonInstance;

pub async fn load(path: &Path) -> Result<HashMap<String, AddonInstance>> {
    if !path.exists() {
        tracing::info!(
            "Addon registry file not found, starting empty: {}",
            path.display()
        );
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| Error::plugin(format!("Failed to read addon registry: {}", e)))?;
    let instances: HashMap<String, AddonInstance> = serde_json::from_str(&content)
        .map_err(|e| Error::plugin(format!("Failed to parse addon registry: {}", e)))?;
    Ok(instances)
}

pub async fn save(path: &Path, instances: &HashMap<String, AddonInstance>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::plugin(format!("Failed to create addon registry dir: {}", e)))?;
    }
    let content = serde_json::to_string_pretty(instances)
        .map_err(|e| Error::plugin(format!("Failed to serialize addon registry: {}", e)))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &content)
        .await
        .map_err(|e| Error::plugin(format!("Failed to write addon registry: {}", e)))?;
    fs::rename(&tmp, path)
        .await
        .map_err(|e| Error::plugin(format!("Failed to finalize addon registry write: {}", e)))?;
    Ok(())
}
