use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub swagger_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig { swagger_url: None },
        }
    }
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        // Use ~/.config instead of platform-specific directory
        let home_dir = dirs::home_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not find home directory"))?;

        let config_dir = home_dir.join(".config");
        let dotrest_dir = config_dir.join("dotrest");

        // Create directory if it doesn't exist
        if !dotrest_dir.exists() {
            fs::create_dir_all(&dotrest_dir)?;
        }

        Ok(dotrest_dir.join("config.toml"))
    }

    /// Load config from file, or return default if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&contents)?;

        Ok(config)
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(&config_path, toml_string)?;
        Ok(())
    }

    /// Set swagger URL and save
    pub fn set_swagger_url(&mut self, url: String) -> Result<()> {
        self.server.swagger_url = Some(url);
        self.save()?;
        Ok(())
    }
}

/// Simple URL validation
pub fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    // Check for basic URL structure
    if !url.contains("://") {
        return Err("Invalid URL format".to_string());
    }

    Ok(())
}
