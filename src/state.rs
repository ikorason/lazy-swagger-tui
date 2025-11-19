use crate::types::{
    ApiEndpoint, ApiResponse, AuthState, DetailsPanelFocus, InputMode, LoadingState, PanelFocus,
    RenderItem, UrlInputField, ViewMode,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct AppState {
    pub endpoints: Vec<ApiEndpoint>,
    pub loading_state: LoadingState,
    pub retry_count: u32,
    pub grouped_endpoints: HashMap<String, Vec<ApiEndpoint>>,
    pub view_mode: ViewMode,
    pub expanded_groups: HashSet<String>,
    pub render_items: Vec<RenderItem>,
    pub auth: AuthState,
    pub input_mode: InputMode,
    pub token_input: String,
    pub url_input: String,
    pub base_url_input: String,
    /// track which field is active
    pub active_url_field: UrlInputField,
    /// path of endpoints currently being executed
    pub executing_endpoint: Option<String>,
    /// Response for currently selected endpoint
    pub current_response: Option<ApiResponse>,
    /// Tracks which sections in the response panel are expanded/collapsed
    /// This is global state shared across all endpoint responses
    pub response_sections_expanded: ResponseSectionsState,

    /// Which main panel (left or right) has focus
    pub panel_focus: PanelFocus,

    /// Which section of details panel has focus (only relevant when panel_focus = Details)
    pub details_focus: DetailsPanelFocus,

    /// Scroll offset for response body section (lines)
    pub response_body_scroll: usize,

    /// Scroll offset for headers section (lines)
    pub headers_scroll: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            endpoints: Vec::new(),
            loading_state: LoadingState::Idle,
            retry_count: 0,
            grouped_endpoints: HashMap::new(),
            view_mode: ViewMode::Grouped,
            expanded_groups: HashSet::new(),
            render_items: Vec::new(),
            auth: AuthState::new(),
            input_mode: InputMode::Normal,
            token_input: String::new(),
            url_input: String::new(),
            base_url_input: String::new(),
            active_url_field: UrlInputField::SwaggerUrl,
            executing_endpoint: None,
            current_response: None,
            response_sections_expanded: ResponseSectionsState::default(),
            panel_focus: PanelFocus::EndpointsList,
            details_focus: DetailsPanelFocus::ResponseBody, // Start focused on response
            response_body_scroll: 0,
            headers_scroll: 0,
        }
    }
}

/// Helper function to count visible items in current view mode
pub fn count_visible_items(state: &AppState) -> usize {
    match state.view_mode {
        ViewMode::Flat => state.endpoints.len(),
        ViewMode::Grouped => {
            let mut count = 0;
            let mut group_names: Vec<&String> = state.grouped_endpoints.keys().collect();
            group_names.sort();

            for group_name in group_names {
                count += 1; // Group header
                if state.expanded_groups.contains(group_name) {
                    let endpoints = &state.grouped_endpoints[group_name];
                    count += endpoints.len();
                }
            }
            count
        }
    }
}

/// Tracks which sections of the response panel are expanded/collapsed
/// This is global state - applies to all endpoints
#[derive(Debug, Clone)]
pub struct ResponseSectionsState {
    /// Whether "Endpoint Details" section is expanded
    pub endpoint_details: bool,

    /// Whether "Response Body" section is expanded
    pub response_body: bool,

    /// Whether "Response Headers" section is expanded
    pub response_headers: bool,
}

impl Default for ResponseSectionsState {
    fn default() -> Self {
        Self {
            endpoint_details: true,  // Show endpoint info by default
            response_body: true,     // Show response by default
            response_headers: false, // Hide headers by default (reduce noise)
        }
    }
}
