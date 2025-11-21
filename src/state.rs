use crate::types::{
    ApiEndpoint, ApiResponse, AuthState, DetailTab, InputMode, LoadingState, PanelFocus,
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

    /// Which main panel (left or right) has focus
    pub panel_focus: PanelFocus,

    pub active_detail_tab: DetailTab,

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
            panel_focus: PanelFocus::EndpointsList,
            active_detail_tab: DetailTab::Endpoint,
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
