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

    /// Set swagger URL and base URL, then save
    pub fn set_swagger_url(&mut self, swagger_url: String, base_url: Option<String>) -> Result<()> {
        self.server.swagger_url = Some(swagger_url);

        // Use provided base_url as-is (no auto-extraction)
        self.server.base_url = base_url;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_valid_http() {
        assert!(validate_url("http://localhost:5000").is_ok());
    }

    #[test]
    fn test_validate_url_valid_https() {
        assert!(validate_url("https://api.example.com/swagger/v1/swagger.json").is_ok());
    }

    #[test]
    fn test_validate_url_empty() {
        let result = validate_url("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "URL cannot be empty");
    }

    #[test]
    fn test_validate_url_no_protocol() {
        let result = validate_url("localhost:5000");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "URL must start with http:// or https://"
        );
    }

    #[test]
    fn test_validate_url_invalid_protocol() {
        let result = validate_url("ftp://example.com");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "URL must start with http:// or https://"
        );
    }
}
