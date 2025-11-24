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
    // Future additions:
    // pub body: Option<String>,
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
    #[allow(dead_code)]
    ConfirmClearToken,
    EnteringUrl,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UrlInputField {
    SwaggerUrl,
    BaseUrl,
}

#[derive(Debug, Clone)]
pub struct AuthState {
    pub token: Option<String>,
}

impl AuthState {
    pub fn new() -> Self {
        Self { token: None }
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    pub fn clear_token(&mut self) {
        self.token = None;
    }

    pub fn get_masked_display(&self) -> String {
        match &self.token {
            Some(token) => mask_token(token),
            None => "Not set".to_string(),
        }
    }
}

fn mask_token(token: &str) -> String {
    let len = token.len();
    if len <= 15 {
        // Too short to safely show, just show dots
        return "●".repeat(len);
    }

    let first = &token[..7];
    let last = &token[len - 6..];
    format!("{}...{}", first, last)
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
