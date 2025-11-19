use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
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

/// Tracks which section of the details panel has focus for scrolling
#[derive(Debug, Clone, PartialEq)]
pub enum DetailsPanelFocus {
    EndpointDetails,
    ResponseBody,
    Headers,
}
