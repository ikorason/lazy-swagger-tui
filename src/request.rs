use crate::state::AppState;
use crate::types::{ApiEndpoint, ApiResponse};
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
        // Build the full URL
        let full_url = format!("{}{}", base_url.trim_end_matches('/'), endpoint.path);

        // Build and execute request
        let response = execute_get_request(&full_url, &state).await;

        // Store response and clear executing flag
        {
            let mut s = state.write().unwrap();
            s.executing_endpoint = None;
            s.current_response = Some(response);
        }
    });
}

async fn execute_get_request(url: &str, state: &Arc<RwLock<AppState>>) -> ApiResponse {
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

    // Execute request
    match request_builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();

            // Get response body as text
            match response.text().await {
                Ok(body) => ApiResponse {
                    status,
                    body,
                    duration: std::time::Duration::from_secs(0), // Will be set by caller
                    is_error: false,
                    error_message: None,
                },
                Err(e) => ApiResponse {
                    status: 0,
                    body: String::new(),
                    duration: std::time::Duration::from_secs(0),
                    is_error: true,
                    error_message: Some(format!("Failed to read response body: {}", e)),
                },
            }
        }
        Err(e) => {
            // Network error or connection failure
            ApiResponse {
                status: 0,
                body: String::new(),
                duration: std::time::Duration::from_secs(0),
                is_error: true,
                error_message: Some(format!("Request failed: {}", e)),
            }
        }
    }
}
