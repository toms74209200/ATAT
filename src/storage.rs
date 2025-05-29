use crate::config;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Abstract token persistence interface
pub trait TokenStorage {
    /// Return the stored token. If none is stored, returns Ok(None)
    fn load(&self) -> Result<Option<String>>;
    /// Persist the token
    fn save(&self, token: &str) -> Result<()>;
    /// Delete the token
    fn delete(&self) -> Result<()>;
}

/// Abstract configuration persistence interface
pub trait ConfigStorage {
    /// Load configuration into a HashMap.
    /// This method should handle parsing of the configuration file content.
    fn load_config(&self) -> Result<HashMap<config::ConfigKey, Value>>;

    /// Save the given configuration HashMap.
    /// This method should handle serializing the HashMap to the appropriate format before writing.
    fn save_config(&self, config_data: &HashMap<config::ConfigKey, Value>) -> Result<()>;
}

/// File-based token persistence implementation
pub struct FileTokenStorage {
    path: PathBuf,
}

impl FileTokenStorage {
    pub fn new() -> Self {
        let mut dir = std::env::var_os("HOME")
            .map(PathBuf::from)
            .expect("HOME environment variable not set");
        dir.push(".atat");
        let _ = fs::create_dir_all(&dir);
        dir.push("token");
        FileTokenStorage { path: dir }
    }
}

impl Default for FileTokenStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStorage for FileTokenStorage {
    fn load(&self) -> Result<Option<String>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.path).context("Failed to read token file")?;
        Ok(Some(content.trim().to_string()))
    }

    fn save(&self, token: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).context("Failed to create storage directory")?;
        }
        let mut file = File::create(&self.path).context("Failed to open token file for writing")?;
        file.write_all(token.as_bytes())
            .context("Failed to write token to file")?;
        Ok(())
    }

    fn delete(&self) -> Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path).context("Failed to delete token file")?;
        }
        Ok(())
    }
}

/// File-based local configuration persistence implementation
pub struct LocalConfigStorage {
    config_path: PathBuf,
    config_dir: PathBuf,
}

impl LocalConfigStorage {
    pub fn new() -> Result<Self> {
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        let config_dir = current_dir.join(config::PROJECT_CONFIG_DIR);
        let config_path = config_dir.join(config::PROJECT_CONFIG_FILENAME);
        Ok(LocalConfigStorage {
            config_path,
            config_dir,
        })
    }
}

impl ConfigStorage for LocalConfigStorage {
    fn load_config(&self) -> Result<HashMap<config::ConfigKey, Value>> {
        let content = read_file_bytes(&self.config_path).context(format!(
            "Failed to read project config file at {:?}",
            self.config_path
        ))?;
        config::parse_config(&content)
    }

    fn save_config(&self, config_data: &HashMap<config::ConfigKey, Value>) -> Result<()> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir).context(format!(
                "Failed to create project config directory at {:?}",
                self.config_dir
            ))?;
        }

        let mut json_map = serde_json::Map::new();
        for (key, value) in config_data {
            json_map.insert(key.as_str().to_string(), value.clone());
        }
        let content_str = serde_json::to_string_pretty(&json_map)
            .context("Failed to serialize config to JSON for saving")?;

        let mut file = File::create(&self.config_path).context(format!(
            "Failed to open project config file for writing at {:?}",
            self.config_path
        ))?;
        file.write_all(content_str.as_bytes()).context(format!(
            "Failed to write to project config file at {:?}",
            self.config_path
        ))?;
        Ok(())
    }
}

/// Reads the content of the file at the specified path into a byte vector.
///
/// - Returns `Ok(Vec::new())` if the file does not exist.
/// - Returns an `Err` if any other error occurs during reading (e.g., permission denied).
fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
    match fs::read(path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).context(format!("Failed to read file: {:?}", path)),
    }
}
