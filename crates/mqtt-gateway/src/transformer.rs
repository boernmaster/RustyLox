//! Message transformer system

use rustylox_core::Result;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::RwLock;
use tracing::{debug, info};

/// Transform result
#[derive(Debug, Clone, Serialize)]
pub struct TransformResult {
    pub topic: String,
    pub value: String,
    pub relay_to_miniserver: bool,
    pub relay_to_mqtt: bool,
}

/// Transformer type
pub trait Transformer: Send + Sync {
    fn name(&self) -> &str;
    fn transform(&self, topic: &str, value: &str) -> Result<Option<TransformResult>>;
}

/// JSON expansion transformer
/// Expands JSON payloads into separate topics
pub struct JsonExpansionTransformer;

impl Transformer for JsonExpansionTransformer {
    fn name(&self) -> &str {
        "json_expansion"
    }

    fn transform(&self, topic: &str, value: &str) -> Result<Option<TransformResult>> {
        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
            if json.is_object() {
                // For now, just return the original
                // In a full implementation, this would create multiple messages
                debug!("JSON object detected in {}", topic);
            }
        }

        // Return None to continue pipeline
        Ok(None)
    }
}

/// Boolean conversion transformer
/// Converts various boolean representations to 0/1
pub struct BooleanTransformer;

impl Transformer for BooleanTransformer {
    fn name(&self) -> &str {
        "boolean_conversion"
    }

    fn transform(&self, topic: &str, value: &str) -> Result<Option<TransformResult>> {
        let lower = value.trim().to_lowercase();

        let converted = match lower.as_str() {
            "true" | "on" | "yes" | "1" => Some("1"),
            "false" | "off" | "no" | "0" => Some("0"),
            _ => None,
        };

        if let Some(new_value) = converted {
            debug!("Boolean conversion: {} -> {}", value, new_value);
            return Ok(Some(TransformResult {
                topic: topic.to_string(),
                value: new_value.to_string(),
                relay_to_miniserver: true,
                relay_to_mqtt: false,
            }));
        }

        Ok(None)
    }
}

/// External script transformer
/// Executes external Perl/PHP/Bash scripts
pub struct ScriptTransformer {
    name: String,
    path: PathBuf,
}

impl ScriptTransformer {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }
}

impl Transformer for ScriptTransformer {
    fn name(&self) -> &str {
        &self.name
    }

    fn transform(&self, topic: &str, value: &str) -> Result<Option<TransformResult>> {
        // This would execute the script - simplified for now
        debug!(
            "Would execute transformer: {} {} {}",
            self.path.display(),
            topic,
            value
        );
        Ok(None)
    }
}

/// Transformer registry
pub struct TransformerRegistry {
    transform_dir: PathBuf,
    transformers: RwLock<Vec<Box<dyn Transformer>>>,
}

impl TransformerRegistry {
    /// Create a new transformer registry
    pub fn new(transform_dir: PathBuf) -> Self {
        Self {
            transform_dir,
            transformers: RwLock::new(Vec::new()),
        }
    }

    /// Load transformers from disk
    pub async fn load(&self) -> Result<()> {
        let mut transformers: Vec<Box<dyn Transformer>> = Vec::new();

        // Add built-in transformers
        transformers.push(Box::new(JsonExpansionTransformer));
        transformers.push(Box::new(BooleanTransformer));

        // Load external transformers from shipped directory
        let shipped_dir = self.transform_dir.join("shipped");
        if shipped_dir.exists() {
            if let Ok(entries) = tokio::fs::read_dir(&shipped_dir).await {
                let mut entries = entries;
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            info!("Found transformer: {}", name);
                            transformers
                                .push(Box::new(ScriptTransformer::new(name.to_string(), path)));
                        }
                    }
                }
            }
        }

        // Load custom transformers
        let custom_dir = self.transform_dir.join("custom");
        if custom_dir.exists() {
            if let Ok(entries) = tokio::fs::read_dir(&custom_dir).await {
                let mut entries = entries;
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            info!("Found custom transformer: {}", name);
                            transformers.push(Box::new(ScriptTransformer::new(
                                format!("custom_{}", name),
                                path,
                            )));
                        }
                    }
                }
            }
        }

        let mut registry = self.transformers.write().unwrap();
        *registry = transformers;

        info!("Loaded {} transformers", registry.len());

        Ok(())
    }

    /// Apply transformers to a message
    pub async fn transform(&self, topic: &str, value: &str) -> Result<TransformResult> {
        let transformers = self.transformers.read().unwrap();

        // Apply transformers in sequence
        for transformer in transformers.iter() {
            if let Some(result) = transformer.transform(topic, value)? {
                debug!("Transformer '{}' modified message", transformer.name());
                return Ok(result);
            }
        }

        // No transformer modified the message, return original
        Ok(TransformResult {
            topic: topic.to_string(),
            value: value.to_string(),
            relay_to_miniserver: true,
            relay_to_mqtt: false,
        })
    }

    /// Count loaded transformers
    pub fn count(&self) -> usize {
        self.transformers.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_transformer() {
        let transformer = BooleanTransformer;

        let result = transformer.transform("test/topic", "true").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, "1");

        let result = transformer.transform("test/topic", "OFF").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, "0");

        let result = transformer.transform("test/topic", "123").unwrap();
        assert!(result.is_none());
    }
}
