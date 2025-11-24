use url::Url;

use crate::state::AppState;
use crate::types::{ApiEndpoint, ApiResponse};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Executes an HTTP request for the given endpoint in the background
pub fn execute_request_background(
    state: Arc<RwLock<AppState>>,
    endpoint: ApiEndpoint,
    base_url: String,
) {
    // Mark this endpoint as executing
    {
        let mut s = state.write().unwrap();
        s.executing_endpoint = Some(endpoint.path.clone());
        s.current_response = None; // Clear any previous response
    }

    // Spawn background task
    tokio::spawn(async move {
        // Get path and query parameters from request config
        let (path_params, query_params) = {
            let s = state.read().unwrap();
            s.request_configs
                .get(&endpoint.path)
                .map(|config| (config.path_params.clone(), config.query_params.clone()))
                .unwrap_or_default()
        };

        // Build the full URL with query parameters
        let full_url =
            match build_url_with_params(&base_url, &endpoint.path, &path_params, &query_params) {
                Ok(url) => url,
                Err(e) => {
                    // Handle URL building error
                    let mut s = state.write().unwrap();
                    s.executing_endpoint = None;
                    s.current_response = Some(ApiResponse::error(format!("Failed to build URL: {}", e)));
                    return;
                }
            };

        // Build and execute request
        let response = execute_get_request(&full_url, &state).await;

        // Store response and clear executing flag
        {
            let mut s = state.write().unwrap();
            s.executing_endpoint = None;
            s.current_response = Some(response);
            s.response_body_scroll = 0; // Reset to top
            s.headers_scroll = 0; // Reset to top
        }
    });
}

async fn execute_get_request(url: &str, state: &Arc<RwLock<AppState>>) -> ApiResponse {
    use std::time::Instant; // Add this import at the top

    // Get auth token if available
    let token = {
        let s = state.read().unwrap();
        s.auth.token.clone()
    };

    // Build request
    let client = reqwest::Client::new();
    let mut request_builder = client.get(url);

    // Add bearer token if available
    if let Some(token) = token {
        request_builder = request_builder.bearer_auth(token);
    }

    // Start timing the request
    let start = Instant::now();

    // Execute request
    match request_builder.send().await {
        Ok(response) => {
            let duration = start.elapsed(); // Capture duration immediately

            let status = response.status().as_u16();
            let status_text = response
                .status()
                .canonical_reason()
                .unwrap_or("Unknown")
                .to_string();

            // Extract headers (normalize keys to lowercase for consistency)
            let headers: HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(key, value)| {
                    (
                        key.as_str().to_lowercase(),
                        value.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();

            // Get response body as text
            match response.text().await {
                Ok(body) => ApiResponse {
                    status,
                    status_text,
                    headers,
                    body,
                    duration, // Use actual measured duration
                    is_error: false,
                    error_message: None,
                },
                Err(e) => ApiResponse {
                    status: 0,
                    status_text: String::new(),
                    headers: HashMap::new(),
                    body: String::new(),
                    duration, // Even on error, show how long we waited
                    is_error: true,
                    error_message: Some(format!("Failed to read response body: {}", e)),
                },
            }
        }
        Err(e) => {
            let duration = start.elapsed(); // Capture duration for failed requests too

            // Network error or connection failure (didn't get HTTP response)
            ApiResponse {
                status: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                duration,
                is_error: true,
                error_message: Some(format!("Request failed: {}", e)),
            }
        }
    }
}

/// Build a full URL with path and query parameters
fn build_url_with_params(
    base_url: &str,
    path_template: &str,
    path_params: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
) -> Result<String, String> {
    // Step 1: Substitute path parameters
    let mut path = path_template.to_string();

    for (key, value) in path_params {
        let placeholder = format!("{{{}}}", key);
        if path.contains(&placeholder) {
            path = path.replace(&placeholder, value);
        }
    }

    // Step 2: Build full URL with base
    let full_path = format!("{}{}", base_url.trim_end_matches('/'), path);

    // Step 3: Parse as URL
    let mut url = Url::parse(&full_path).map_err(|e| format!("Invalid URL: {}", e))?;

    // Step 4: Add query parameters (only non-empty ones)
    for (key, value) in query_params {
        if !value.is_empty() {
            url.query_pairs_mut().append_pair(key, value);
        }
    }

    Ok(url.to_string())
}
