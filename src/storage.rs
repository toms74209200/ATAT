use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

/// Abstract token persistence interface
pub trait TokenStorage {
    /// Return the stored token. If none is stored, returns Ok(None)
    fn load(&self) -> Result<Option<String>>;
    /// Persist the token
    fn save(&self, token: &str) -> Result<()>;
    /// Delete the token
    fn delete(&self) -> Result<()>;
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
