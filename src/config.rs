use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Configuration keys enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigKey {
    Repositories,
}

impl ConfigKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigKey::Repositories => "repositories",
        }
    }

    /// Get all config keys
    pub fn all() -> &'static [ConfigKey] {
        &[ConfigKey::Repositories]
    }
}

/// Filename for the project-specific configuration within the .atat directory.
pub const PROJECT_CONFIG_FILENAME: &str = "config.json";
/// Directory name for project-specific configuration.
pub const PROJECT_CONFIG_DIR: &str = ".atat";

/// Parses a JSON configuration file content into a map of configuration values.
///
/// Expects `content` to be a byte slice representing either:
/// - A simple JSON array of strings (treated as just the repositories list)
/// - A JSON object with configuration keys (e.g., {"repositories": [...], ...})
///
/// - Returns `Ok(HashMap<ConfigKey, Value>)` containing all parsed configuration values.
/// - Returns `Ok(HashMap with only repositories)` if input is a direct array.
/// - Returns an empty HashMap if the input `content` is empty or contains only whitespace.
/// - Returns an `Err` if the JSON parsing fails (e.g., invalid format).
pub fn parse_config(content: &[u8]) -> Result<HashMap<ConfigKey, Value>> {
    if content.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(HashMap::new());
    }

    let value: Value = serde_json::from_slice(content).context("Failed to parse config JSON")?;

    let mut config_map = HashMap::new();

    if let Value::Object(map) = &value {
        for key in ConfigKey::all() {
            if let Some(val) = map.get(key.as_str()) {
                config_map.insert(*key, val.clone());
            }
        }
        return Ok(config_map);
    }

    Err(anyhow::anyhow!(
        "Config must be either an array of strings or an object"
    ))
}

/// Merges `updates` into `base_config` and returns a new configuration map.
///
/// - Keys from `updates` are added to a clone of `base_config`.
/// - If a key exists in both, the value from `updates` overwrites the value in the cloned `base_config`.
///
/// # Arguments
///
/// * `base_config`: A reference to the base configuration map.
/// * `updates`: A reference to the configuration map containing updates to apply.
///
/// # Returns
///
/// * A new `HashMap<ConfigKey, Value>` representing the merged configuration.
pub fn update_config(
    base_config: &HashMap<ConfigKey, Value>,
    updates: &HashMap<ConfigKey, Value>,
) -> HashMap<ConfigKey, Value> {
    let mut new_config = base_config.clone();
    for (key, value) in updates {
        new_config.insert(*key, value.clone());
    }
    new_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_with_key_works() {
        let json = r#"{"repositories": ["owner/repo1", "another/repo2"]}"#.as_bytes();
        let config = parse_config(json).unwrap();
        assert!(config.contains_key(&ConfigKey::Repositories));

        let repos = config.get(&ConfigKey::Repositories).unwrap();
        assert_eq!(repos, &json!(["owner/repo1", "another/repo2"]));
    }

    #[test]
    fn empty_object_key_works() {
        let json = r#"{"repositories": []}"#.as_bytes();
        let config = parse_config(json).unwrap();
        assert!(config.contains_key(&ConfigKey::Repositories));

        let repos = config.get(&ConfigKey::Repositories).unwrap();
        assert_eq!(repos, &json!([]));
    }

    #[test]
    fn empty_input_works() {
        let json = r#""#.as_bytes();
        let config = parse_config(json).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn whitespace_input_works() {
        let json = r#"   "#.as_bytes();
        let config = parse_config(json).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn invalid_json_fails() {
        let json = r#"["owner/repo1""#.as_bytes();
        assert!(parse_config(json).is_err());
    }

    #[test]
    fn invalid_syntax_fails() {
        let json = r#"{invalid json}"#.as_bytes();
        assert!(parse_config(json).is_err());
    }

    #[test]
    fn unknown_key_skipped() {
        let json = r#"{"unknown": "value"}"#.as_bytes();
        let config = parse_config(json).unwrap();
        assert!(!config.contains_key(&ConfigKey::Repositories));
    }

    #[test]
    fn test_update_config_add_new_key_pure() {
        let base_config = HashMap::new();
        let mut updates = HashMap::new();
        updates.insert(ConfigKey::Repositories, json!(["owner/repo1"]));

        let result_config = update_config(&base_config, &updates);
        assert_eq!(
            result_config.get(&ConfigKey::Repositories).unwrap(),
            &json!(["owner/repo1"])
        );
        assert!(base_config.is_empty()); // Ensure base_config is not modified
    }

    #[test]
    fn test_update_config_overwrite_existing_key_pure() {
        let mut base_config = HashMap::new();
        base_config.insert(ConfigKey::Repositories, json!(["owner/repo1"]));
        let mut updates = HashMap::new();
        updates.insert(ConfigKey::Repositories, json!(["owner/repo2"]));

        let original_base_clone = base_config.clone();
        let result_config = update_config(&base_config, &updates);

        assert_eq!(
            result_config.get(&ConfigKey::Repositories).unwrap(),
            &json!(["owner/repo2"])
        );
        assert_eq!(base_config, original_base_clone); // Ensure base_config is not modified
    }

    #[test]
    fn test_update_config_overwrite_with_different_type_pure() {
        let mut base_config = HashMap::new();
        base_config.insert(ConfigKey::Repositories, json!(["owner/repo1"]));
        let mut updates = HashMap::new();
        updates.insert(ConfigKey::Repositories, json!("owner/repo2"));

        let original_base_clone = base_config.clone();
        let result_config = update_config(&base_config, &updates);

        assert_eq!(
            result_config.get(&ConfigKey::Repositories).unwrap(),
            &json!("owner/repo2")
        );
        assert_eq!(base_config, original_base_clone); // Ensure base_config is not modified
    }

    #[test]
    fn test_update_config_empty_updates_pure() {
        let mut base_config = HashMap::new();
        base_config.insert(ConfigKey::Repositories, json!(["owner/repo1"]));
        let updates = HashMap::new();

        let original_base_clone = base_config.clone();
        let result_config = update_config(&base_config, &updates);

        assert_eq!(
            result_config.get(&ConfigKey::Repositories).unwrap(),
            &json!(["owner/repo1"])
        );
        assert_eq!(base_config, original_base_clone);
    }

    #[test]
    fn test_update_config_empty_base_pure() {
        let base_config = HashMap::new();
        let mut updates = HashMap::new();
        updates.insert(ConfigKey::Repositories, json!(["owner/repo1"]));

        let result_config = update_config(&base_config, &updates);
        assert_eq!(
            result_config.get(&ConfigKey::Repositories).unwrap(),
            &json!(["owner/repo1"])
        );
        assert!(base_config.is_empty());
    }
}
