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
                    s.current_response =
                        Some(ApiResponse::error(format!("Failed to build URL: {}", e)));
                    return;
                }
            };

        // Convert method string to reqwest::Method
        let method = match endpoint.method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "PATCH" => reqwest::Method::PATCH,
            "DELETE" => reqwest::Method::DELETE,
            _ => reqwest::Method::GET, // Default to GET for unknown methods
        };

        // Build and execute request
        let response = execute_request(&full_url, method, &state).await;

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

async fn execute_request(
    url: &str,
    method: reqwest::Method,
    state: &Arc<RwLock<AppState>>,
) -> ApiResponse {
    use std::time::Instant;

    // Get auth token if available
    let token = {
        let s = state.read().unwrap();
        s.auth.token.clone()
    };

    // Build request with the appropriate HTTP method
    let client = reqwest::Client::new();
    let mut request_builder = client.request(method.clone(), url);

    // Add empty body for methods that typically require it
    if method == reqwest::Method::POST
        || method == reqwest::Method::PUT
        || method == reqwest::Method::PATCH
    {
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .body("{}");
    }

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
pub(crate) fn build_url_with_params(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_basic() {
        let url = build_url_with_params(
            "http://localhost:5000",
            "/users",
            &HashMap::new(),
            &HashMap::new(),
        );
        assert_eq!(url.unwrap(), "http://localhost:5000/users");
    }

    #[test]
    fn test_build_url_with_single_path_param() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), "123".to_string());

        let url = build_url_with_params(
            "http://localhost:5000",
            "/users/{id}",
            &path_params,
            &HashMap::new(),
        );
        assert_eq!(url.unwrap(), "http://localhost:5000/users/123");
    }

    #[test]
    fn test_build_url_with_multiple_path_params() {
        let mut path_params = HashMap::new();
        path_params.insert("userId".to_string(), "42".to_string());
        path_params.insert("postId".to_string(), "99".to_string());

        let url = build_url_with_params(
            "http://localhost:5000",
            "/users/{userId}/posts/{postId}",
            &path_params,
            &HashMap::new(),
        );
        assert_eq!(url.unwrap(), "http://localhost:5000/users/42/posts/99");
    }

    #[test]
    fn test_build_url_with_single_query_param() {
        let mut query_params = HashMap::new();
        query_params.insert("limit".to_string(), "10".to_string());

        let url = build_url_with_params(
            "http://localhost:5000",
            "/users",
            &HashMap::new(),
            &query_params,
        );
        assert_eq!(url.unwrap(), "http://localhost:5000/users?limit=10");
    }

    #[test]
    fn test_build_url_with_multiple_query_params() {
        let mut query_params = HashMap::new();
        query_params.insert("skip".to_string(), "20".to_string());
        query_params.insert("limit".to_string(), "10".to_string());

        let url = build_url_with_params(
            "http://localhost:5000",
            "/users",
            &HashMap::new(),
            &query_params,
        )
        .unwrap();

        // Query params order is not guaranteed, so check both possibilities
        assert!(
            url == "http://localhost:5000/users?skip=20&limit=10"
                || url == "http://localhost:5000/users?limit=10&skip=20"
        );
    }

    #[test]
    fn test_build_url_with_path_and_query_params() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), "123".to_string());

        let mut query_params = HashMap::new();
        query_params.insert("include".to_string(), "profile".to_string());

        let url = build_url_with_params(
            "http://localhost:5000",
            "/users/{id}",
            &path_params,
            &query_params,
        );
        assert_eq!(
            url.unwrap(),
            "http://localhost:5000/users/123?include=profile"
        );
    }

    #[test]
    fn test_build_url_empty_query_params_ignored() {
        let mut query_params = HashMap::new();
        query_params.insert("limit".to_string(), "10".to_string());
        query_params.insert("filter".to_string(), "".to_string()); // Empty value

        let url = build_url_with_params(
            "http://localhost:5000",
            "/users",
            &HashMap::new(),
            &query_params,
        );
        assert_eq!(url.unwrap(), "http://localhost:5000/users?limit=10");
    }

    #[test]
    fn test_build_url_with_trailing_slash_in_base() {
        let url = build_url_with_params(
            "http://localhost:5000/",
            "/users",
            &HashMap::new(),
            &HashMap::new(),
        );
        assert_eq!(url.unwrap(), "http://localhost:5000/users");
    }

    #[test]
    fn test_build_url_special_chars_in_query_params() {
        let mut query_params = HashMap::new();
        query_params.insert("search".to_string(), "hello world".to_string());

        let url = build_url_with_params(
            "http://localhost:5000",
            "/users",
            &HashMap::new(),
            &query_params,
        );
        // URL encoding should happen automatically
        assert_eq!(
            url.unwrap(),
            "http://localhost:5000/users?search=hello+world"
        );
    }

    #[test]
    fn test_build_url_path_param_not_in_template() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), "123".to_string());
        path_params.insert("unused".to_string(), "value".to_string());

        let url = build_url_with_params(
            "http://localhost:5000",
            "/users/{id}",
            &path_params,
            &HashMap::new(),
        );
        // Should still work, unused params are ignored
        assert_eq!(url.unwrap(), "http://localhost:5000/users/123");
    }

    #[test]
    fn test_build_url_missing_path_param() {
        let url = build_url_with_params(
            "http://localhost:5000",
            "/users/{id}",
            &HashMap::new(),
            &HashMap::new(),
        );
        // Path placeholder remains unreplaced, but gets URL encoded by the Url parser
        assert_eq!(url.unwrap(), "http://localhost:5000/users/%7Bid%7D");
    }

    #[test]
    fn test_build_url_invalid_base() {
        let url = build_url_with_params(
            "not a valid url",
            "/users",
            &HashMap::new(),
            &HashMap::new(),
        );
        assert!(url.is_err());
        assert!(url.unwrap_err().contains("Invalid URL"));
    }
}
