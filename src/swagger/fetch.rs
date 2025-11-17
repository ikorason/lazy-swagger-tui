use crate::state::AppState;
use crate::swagger::parse::parse_swagger_spec;
use crate::types::{ApiEndpoint, LoadingState, SwaggerSpec};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Spawns a background task to fetch endpoints
pub fn fetch_endpoints_background(state: Arc<RwLock<AppState>>, url: String) {
    // Set loading state
    if let Ok(mut s) = state.write() {
        s.loading_state = LoadingState::Fetching;
    }

    tokio::spawn(async move {
        match reqwest::get(&url).await {
            Ok(response) => {
                if let Ok(mut s) = state.write() {
                    s.loading_state = LoadingState::Parsing;
                }

                match response.json::<SwaggerSpec>().await {
                    Ok(spec) => {
                        let endpoints = parse_swagger_spec(spec);

                        // Group endpoints
                        let mut grouped: HashMap<String, Vec<ApiEndpoint>> = HashMap::new();
                        for endpoint in &endpoints {
                            if endpoint.tags.is_empty() {
                                grouped
                                    .entry("Other".to_string())
                                    .or_default()
                                    .push(endpoint.clone());
                            } else {
                                for tag in &endpoint.tags {
                                    grouped
                                        .entry(tag.clone())
                                        .or_default()
                                        .push(endpoint.clone());
                                }
                            }
                        }

                        if let Ok(mut s) = state.write() {
                            s.endpoints = endpoints;
                            s.grouped_endpoints = grouped;
                            s.loading_state = LoadingState::Complete;
                            s.retry_count = 0;
                        }
                    }
                    Err(e) => {
                        if let Ok(mut s) = state.write() {
                            s.loading_state = LoadingState::Error(format!("Parse error: {}", e));
                        }
                    }
                }
            }
            Err(e) => {
                if let Ok(mut s) = state.write() {
                    s.loading_state = LoadingState::Error(format!("Network error: {}", e));
                }
            }
        }
    });
}
