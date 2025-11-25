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
    //* API base URL for requests */
    pub base_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                swagger_url: None,
                base_url: None,
            },
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
        let app_dir = config_dir.join("lazy-swagger-tui");

        // Create directory if it doesn't exist
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir)?;
        }

        Ok(app_dir.join("config.toml"))
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

    /// Set swagger URL, auto-extract base URL, and save
    pub fn set_swagger_url(&mut self, swagger_url: String, base_url: Option<String>) -> Result<()> {
        self.server.swagger_url = Some(swagger_url.clone());

        // Use provided base_url, or extract from swagger_url
        self.server.base_url = base_url.or_else(|| Some(extract_base_url(&swagger_url)));

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

/// Extracts base URL from swagger URL
/// Example: http://localhost:5000/swagger/v1/swagger.json -> http://localhost:5000
pub fn extract_base_url(swagger_url: &str) -> String {
    // Parse the URL
    if let Ok(parsed) = url::Url::parse(swagger_url) {
        // Get scheme, host, and port
        let scheme = parsed.scheme();
        let host = parsed.host_str().unwrap_or("localhost");

        let base = if let Some(port) = parsed.port() {
            format!("{}://{}:{}", scheme, host, port)
        } else {
            format!("{}://{}", scheme, host)
        };

        base
    } else {
        // Fallback: just return the swagger URL if parsing fails
        swagger_url.to_string()
    }
}
