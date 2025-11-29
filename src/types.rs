use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<Parameter>,
}

impl ApiEndpoint {
    /// Get all path parameters for this endpoint
    pub fn path_params(&self) -> Vec<&Parameter> {
        self.parameters
            .iter()
            .filter(|p| p.location == "path")
            .collect()
    }

    /// Get all query parameters for this endpoint
    pub fn query_params(&self) -> Vec<&Parameter> {
        self.parameters
            .iter()
            .filter(|p| p.location == "query")
            .collect()
    }

    /// Check if all required path parameters have values in the given config
    pub fn has_all_required_path_params(&self, config: &RequestConfig) -> bool {
        self.path_params().iter().all(|param| {
            // Path params are typically always required
            // Check if we have a non-empty value for this param
            config
                .path_params
                .get(&param.name)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
        })
    }

    /// Get list of missing path parameter names
    pub fn missing_path_params(&self, config: &RequestConfig) -> Vec<String> {
        self.path_params()
            .iter()
            .filter(|param| {
                config
                    .path_params
                    .get(&param.name)
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
            })
            .map(|param| param.name.clone())
            .collect()
    }

    /// Check if this endpoint supports request body (POST/PUT/PATCH)
    pub fn supports_body(&self) -> bool {
        matches!(
            self.method.to_uppercase().as_str(),
            "POST" | "PUT" | "PATCH"
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Parameter {
    pub name: String,

    #[serde(rename = "in")]
    pub location: String, // "query", "path", "header", etc.

    pub required: Option<bool>,

    pub schema: Option<ParameterSchema>,

    #[allow(dead_code)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParameterSchema {
    #[serde(rename = "type")]
    pub param_type: Option<String>, // "string", "integer", "boolean"

    pub format: Option<String>, // "int32", "int64", "date-time", etc.

    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct RequestConfig {
    pub query_params: HashMap<String, String>,
    pub path_params: HashMap<String, String>,
    pub body: Option<String>,
    // Future additions:
    // pub headers: HashMap<String, String>,
}

/// Represents an HTTP response from an API endpoint
#[derive(Debug, Clone)]
pub struct ApiResponse {
    /// HTTP status code (200, 404, etc.)
    pub status: u16,

    /// Human-readable status text ("OK", "Not Found", etc.)
    pub status_text: String,

    /// Response headers as key-value pairs (keys normalized to lowercase)
    pub headers: HashMap<String, String>,

    /// Raw response body (could be JSON, HTML, plain text, etc.)
    pub body: String,

    /// Time taken to complete the request
    pub duration: Duration,

    /// True if this was a network error (timeout, connection refused, etc.)
    /// False if we got an HTTP response (even if 4xx/5xx)
    pub is_error: bool,

    /// Error message for network-level failures (only set when is_error = true)
    pub error_message: Option<String>,
}

impl ApiResponse {
    /// Creates an error response with the given error message
    pub fn error(error_message: String) -> Self {
        Self {
            status: 0,
            status_text: String::new(),
            headers: HashMap::new(),
            body: String::new(),
            duration: Duration::from_secs(0),
            is_error: true,
            error_message: Some(error_message),
        }
    }
}

#[derive(Deserialize)]
pub struct SwaggerSpec {
    pub paths: HashMap<String, PathItem>,
}

#[derive(Deserialize)]
pub struct PathItem {
    pub get: Option<Operation>,
    pub post: Option<Operation>,
    pub put: Option<Operation>,
    pub delete: Option<Operation>,
    pub patch: Option<Operation>,
}

#[derive(Deserialize)]
pub struct Operation {
    pub summary: Option<String>,
    pub tags: Option<Vec<String>>,
    pub parameters: Option<Vec<Parameter>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Flat,
    Grouped,
}

#[derive(Debug, Clone)]
pub enum LoadingState {
    Idle,
    Fetching,
    Parsing,
    Complete,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum RenderItem {
    GroupHeader {
        name: String,
        count: usize,
        expanded: bool,
    },
    Endpoint {
        endpoint: ApiEndpoint,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    EnteringToken,
    ConfirmClearToken,
    EnteringUrl,
    Searching,
    EnteringBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UrlInputField {
    SwaggerUrl,
    BaseUrl,
}

#[derive(Debug, Clone)]
pub struct UrlSubmission {
    pub swagger_url: String,
    pub base_url: Option<String>,
}

/// Tracks which main panel has focus
#[derive(Debug, Clone, PartialEq)]
pub enum PanelFocus {
    EndpointsList, // Left panel
    Details,       // Right panel
}

/// Tracks which tab is active in the Details panel
#[derive(Debug, Clone, PartialEq)]
pub enum DetailTab {
    Endpoint,
    Request,
    Headers,
    Response,
}

// For tracking UI state in Request tab
#[derive(Debug, Clone, PartialEq)]
pub enum RequestEditMode {
    // Just navigating, not editing
    Viewing,

    // Editing parameter with this name
    Editing(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create test parameters
    fn create_param(name: &str, location: &str, required: bool) -> Parameter {
        Parameter {
            name: name.to_string(),
            location: location.to_string(),
            required: Some(required),
            schema: None,
            description: None,
        }
    }

    #[test]
    fn test_path_params_filter() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![
                create_param("id", "path", true),
                create_param("limit", "query", false),
            ],
        };

        let path_params = endpoint.path_params();
        assert_eq!(path_params.len(), 1);
        assert_eq!(path_params[0].name, "id");
        assert_eq!(path_params[0].location, "path");
    }

    #[test]
    fn test_query_params_filter() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![
                create_param("id", "path", true),
                create_param("limit", "query", false),
                create_param("skip", "query", false),
            ],
        };

        let query_params = endpoint.query_params();
        assert_eq!(query_params.len(), 2);
        assert!(query_params.iter().any(|p| p.name == "limit"));
        assert!(query_params.iter().any(|p| p.name == "skip"));
    }

    #[test]
    fn test_has_all_required_path_params_success() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![create_param("id", "path", true)],
        };

        let mut config = RequestConfig::default();
        config
            .path_params
            .insert("id".to_string(), "123".to_string());

        assert!(endpoint.has_all_required_path_params(&config));
    }

    #[test]
    fn test_has_all_required_path_params_missing() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![create_param("id", "path", true)],
        };

        let config = RequestConfig::default(); // Empty config

        assert!(!endpoint.has_all_required_path_params(&config));
    }

    #[test]
    fn test_has_all_required_path_params_empty_value() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![create_param("id", "path", true)],
        };

        let mut config = RequestConfig::default();
        config.path_params.insert("id".to_string(), "".to_string()); // Empty string

        assert!(!endpoint.has_all_required_path_params(&config));
    }

    #[test]
    fn test_has_all_required_path_params_multiple() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{userId}/posts/{postId}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![
                create_param("userId", "path", true),
                create_param("postId", "path", true),
            ],
        };

        let mut config = RequestConfig::default();
        config
            .path_params
            .insert("userId".to_string(), "42".to_string());
        config
            .path_params
            .insert("postId".to_string(), "99".to_string());

        assert!(endpoint.has_all_required_path_params(&config));
    }

    #[test]
    fn test_missing_path_params_empty() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![create_param("id", "path", true)],
        };

        let mut config = RequestConfig::default();
        config
            .path_params
            .insert("id".to_string(), "123".to_string());

        let missing = endpoint.missing_path_params(&config);
        assert_eq!(missing.len(), 0);
    }

    #[test]
    fn test_missing_path_params_single() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![create_param("id", "path", true)],
        };

        let config = RequestConfig::default(); // Empty config

        let missing = endpoint.missing_path_params(&config);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "id");
    }

    #[test]
    fn test_missing_path_params_multiple() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{userId}/posts/{postId}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![
                create_param("userId", "path", true),
                create_param("postId", "path", true),
            ],
        };

        let mut config = RequestConfig::default();
        config
            .path_params
            .insert("userId".to_string(), "42".to_string());
        // postId is missing

        let missing = endpoint.missing_path_params(&config);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "postId");
    }

    #[test]
    fn test_missing_path_params_empty_value() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            summary: None,
            tags: vec![],
            parameters: vec![create_param("id", "path", true)],
        };

        let mut config = RequestConfig::default();
        config.path_params.insert("id".to_string(), "".to_string()); // Empty string

        let missing = endpoint.missing_path_params(&config);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "id");
    }

    #[test]
    fn test_request_config_default() {
        let config = RequestConfig::default();
        assert_eq!(config.path_params.len(), 0);
        assert_eq!(config.query_params.len(), 0);
    }
}
