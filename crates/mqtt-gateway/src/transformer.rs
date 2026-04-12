//! Message transformer system

use rustylox_core::Result;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::RwLock;
use tracing::{debug, info};

/// Transform result — one entry per outgoing message produced by a transformer.
#[derive(Debug, Clone, Serialize)]
pub struct TransformResult {
    pub topic: String,
    pub value: String,
    pub relay_to_miniserver: bool,
    pub relay_to_mqtt: bool,
}

/// Transformer trait.
///
/// Returns a `Vec` of results:
/// - Empty vec → transformer did not match; registry continues to next transformer.
/// - One entry  → single replacement message.
/// - Many entries → fan-out (e.g. JSON expansion produces one message per flattened key).
pub trait Transformer: Send + Sync {
    fn name(&self) -> &str;
    fn transform(&self, topic: &str, value: &str) -> Result<Vec<TransformResult>>;
}

// ---------------------------------------------------------------------------
// Recursive JSON flattener — matches LoxBerry's Hash::Flatten behaviour:
//   HashDelimiter = '_'   (nested object keys joined with underscore)
//   ArrayDelimiter = '_'  (array indices also joined with underscore)
//   Sub-topics use '/' as the separator from the base topic.
//
// Examples:
//   {"temp": 21}            → topic/temp = 21
//   {"a": {"b": 1}}         → topic/a_b  = 1
//   [1, 2]                  → topic/0    = 1  ,  topic/1 = 2
//   {"r": [10, 20]}         → topic/r_0  = 10 ,  topic/r_1 = 20
// ---------------------------------------------------------------------------
fn flatten_json(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}_{}", prefix, key)
                };
                flatten_json(&new_prefix, val, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for (idx, val) in arr.iter().enumerate() {
                let new_prefix = if prefix.is_empty() {
                    idx.to_string()
                } else {
                    format!("{}_{}", prefix, idx)
                };
                flatten_json(&new_prefix, val, out);
            }
        }
        serde_json::Value::String(s) => out.push((prefix.to_string(), s.clone())),
        serde_json::Value::Number(n) => out.push((prefix.to_string(), n.to_string())),
        serde_json::Value::Bool(b) => out.push((prefix.to_string(), b.to_string())),
        serde_json::Value::Null => out.push((prefix.to_string(), String::new())),
    }
}

// ---------------------------------------------------------------------------
// JSON expansion transformer
// ---------------------------------------------------------------------------

/// Expands JSON object/array payloads into one message per flattened key.
///
/// Matches LoxBerry V1 `expand_json` behaviour:
/// - Nested keys are joined with `_`
/// - Each key becomes a sub-topic appended to the base topic with `/`
pub struct JsonExpansionTransformer;

impl Transformer for JsonExpansionTransformer {
    fn name(&self) -> &str {
        "json_expansion"
    }

    fn transform(&self, topic: &str, value: &str) -> Result<Vec<TransformResult>> {
        let json: serde_json::Value = match serde_json::from_str(value) {
            Ok(v) => v,
            Err(_) => return Ok(vec![]), // Not JSON — pass through unchanged
        };

        // Only expand objects and arrays; leave plain scalars untouched
        if !json.is_object() && !json.is_array() {
            return Ok(vec![]);
        }

        let mut flat: Vec<(String, String)> = Vec::new();
        flatten_json("", &json, &mut flat);

        if flat.is_empty() {
            return Ok(vec![]);
        }

        debug!(
            "Expand JSON: {} keys from topic {}",
            flat.len(),
            topic
        );

        let results = flat
            .into_iter()
            .map(|(key, val)| TransformResult {
                topic: format!("{}/{}", topic, key),
                value: val,
                relay_to_miniserver: true,
                relay_to_mqtt: false,
            })
            .collect();

        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Boolean conversion transformer
// ---------------------------------------------------------------------------

/// Converts various boolean representations to 0/1.
pub struct BooleanTransformer;

impl Transformer for BooleanTransformer {
    fn name(&self) -> &str {
        "boolean_conversion"
    }

    fn transform(&self, topic: &str, value: &str) -> Result<Vec<TransformResult>> {
        let lower = value.trim().to_lowercase();

        let converted = match lower.as_str() {
            "true" | "on" | "yes" => Some("1"),
            "false" | "off" | "no" => Some("0"),
            _ => None,
        };

        if let Some(new_value) = converted {
            debug!("Boolean conversion: {} -> {}", value, new_value);
            return Ok(vec![TransformResult {
                topic: topic.to_string(),
                value: new_value.to_string(),
                relay_to_miniserver: true,
                relay_to_mqtt: false,
            }]);
        }

        Ok(vec![])
    }
}

// ---------------------------------------------------------------------------
// External script transformer
// ---------------------------------------------------------------------------

/// Executes external Perl/PHP/Bash scripts.
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

    fn transform(&self, topic: &str, value: &str) -> Result<Vec<TransformResult>> {
        // Placeholder — full implementation would execute the script
        debug!(
            "Would execute transformer: {} {} {}",
            self.path.display(),
            topic,
            value
        );
        Ok(vec![])
    }
}

// ---------------------------------------------------------------------------
// Transformer registry
// ---------------------------------------------------------------------------

/// Registry that holds all active transformers and applies them as a pipeline.
pub struct TransformerRegistry {
    transform_dir: PathBuf,
    transformers: RwLock<Vec<Box<dyn Transformer>>>,
}

impl TransformerRegistry {
    /// Create a new transformer registry.
    pub fn new(transform_dir: PathBuf) -> Self {
        Self {
            transform_dir,
            transformers: RwLock::new(Vec::new()),
        }
    }

    /// Load transformers from disk.
    pub async fn load(&self) -> Result<()> {
        let mut transformers: Vec<Box<dyn Transformer>> = Vec::new();

        // Built-in transformers
        transformers.push(Box::new(JsonExpansionTransformer));
        transformers.push(Box::new(BooleanTransformer));

        // Shipped external transformers
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

        // Custom external transformers
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

    /// Apply transformers to a message.
    ///
    /// The first transformer that returns a non-empty result wins.
    /// If no transformer matches, the original message is returned unchanged.
    pub async fn transform(&self, topic: &str, value: &str) -> Result<Vec<TransformResult>> {
        let transformers = self.transformers.read().unwrap();

        for transformer in transformers.iter() {
            let results = transformer.transform(topic, value)?;
            if !results.is_empty() {
                debug!("Transformer '{}' processed message", transformer.name());
                return Ok(results);
            }
        }

        // No transformer matched — pass through unchanged
        Ok(vec![TransformResult {
            topic: topic.to_string(),
            value: value.to_string(),
            relay_to_miniserver: true,
            relay_to_mqtt: false,
        }])
    }

    /// Count loaded transformers.
    pub fn count(&self) -> usize {
        self.transformers.read().unwrap().len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_transformer_true_values() {
        let t = BooleanTransformer;
        for val in &["true", "on", "yes", "TRUE", "ON"] {
            let r = t.transform("test/topic", val).unwrap();
            assert_eq!(r.len(), 1, "expected match for '{}'", val);
            assert_eq!(r[0].value, "1");
        }
    }

    #[test]
    fn test_boolean_transformer_false_values() {
        let t = BooleanTransformer;
        for val in &["false", "off", "no", "FALSE", "OFF"] {
            let r = t.transform("test/topic", val).unwrap();
            assert_eq!(r.len(), 1, "expected match for '{}'", val);
            assert_eq!(r[0].value, "0");
        }
    }

    #[test]
    fn test_boolean_transformer_no_match() {
        let t = BooleanTransformer;
        let r = t.transform("test/topic", "123").unwrap();
        assert!(r.is_empty(), "numeric should not match");
    }

    #[test]
    fn test_json_expand_flat_object() {
        let t = JsonExpansionTransformer;
        let r = t
            .transform("home/sensor", r#"{"temp":21,"hum":65}"#)
            .unwrap();
        assert_eq!(r.len(), 2);
        // Both sub-topics should start with home/sensor/
        assert!(r.iter().any(|x| x.topic == "home/sensor/temp" && x.value == "21"));
        assert!(r.iter().any(|x| x.topic == "home/sensor/hum" && x.value == "65"));
    }

    #[test]
    fn test_json_expand_nested_uses_underscore() {
        let t = JsonExpansionTransformer;
        let r = t
            .transform("home/sensor", r#"{"meta":{"unit":"C"}}"#)
            .unwrap();
        assert_eq!(r.len(), 1);
        // Nested key separator is '_', not '/'
        assert_eq!(r[0].topic, "home/sensor/meta_unit");
        assert_eq!(r[0].value, "C");
    }

    #[test]
    fn test_json_expand_array() {
        let t = JsonExpansionTransformer;
        let r = t.transform("home/sensor", r#"[10,20,30]"#).unwrap();
        assert_eq!(r.len(), 3);
        assert!(r.iter().any(|x| x.topic == "home/sensor/0" && x.value == "10"));
        assert!(r.iter().any(|x| x.topic == "home/sensor/1" && x.value == "20"));
        assert!(r.iter().any(|x| x.topic == "home/sensor/2" && x.value == "30"));
    }

    #[test]
    fn test_json_expand_not_json() {
        let t = JsonExpansionTransformer;
        let r = t.transform("home/sensor", "hello").unwrap();
        assert!(r.is_empty(), "plain string should not expand");
    }

    #[test]
    fn test_json_expand_scalar_json() {
        let t = JsonExpansionTransformer;
        // A bare JSON number or string should pass through without expansion
        let r = t.transform("home/sensor", "42").unwrap();
        assert!(r.is_empty());
    }
}
