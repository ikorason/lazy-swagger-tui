use crate::types::{
    ApiEndpoint, ApiResponse, AuthState, DetailTab, InputMode, LoadingState, PanelFocus,
    RenderItem, RequestConfig, RequestEditMode, UrlInputField, ViewMode,
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

    /// Request configurations per endpoint path
    pub request_configs: HashMap<String, RequestConfig>,

    /// Which parameter is currently selected (for j/k navigation)
    pub selected_param_index: usize,

    /// Are we viewing or editing a parameter?
    pub request_edit_mode: RequestEditMode,

    /// Temporary buffer while editing a param value
    pub param_edit_buffer: String,

    /// Search query for filtering endpoints
    pub search_query: String,

    /// Filtered endpoints (only populated when search_query is not empty)
    pub filtered_endpoints: Vec<ApiEndpoint>,

    /// Filtered grouped endpoints (for grouped view)
    pub filtered_grouped_endpoints: HashMap<String, Vec<ApiEndpoint>>,
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
            request_configs: HashMap::new(),
            selected_param_index: 0,
            request_edit_mode: RequestEditMode::Viewing,
            param_edit_buffer: String::new(),
            search_query: String::new(),
            filtered_endpoints: Vec::new(),
            filtered_grouped_endpoints: HashMap::new(),
        }
    }
}

impl AppState {
    /// Get the selected endpoint based on the current view mode and selected index
    pub fn get_selected_endpoint(&self, selected_index: usize) -> Option<&ApiEndpoint> {
        match self.view_mode {
            ViewMode::Flat => self.endpoints.get(selected_index),
            ViewMode::Grouped => {
                self.render_items
                    .get(selected_index)
                    .and_then(|item| match item {
                        RenderItem::Endpoint { endpoint } => Some(endpoint),
                        RenderItem::GroupHeader { .. } => None,
                    })
            }
        }
    }

    /// Get or create request config for an endpoint, initializing with Swagger defaults
    pub fn get_or_create_request_config(&mut self, endpoint: &ApiEndpoint) -> &mut RequestConfig {
        self.request_configs
            .entry(endpoint.path.clone())
            .or_insert_with(|| {
                let mut config = RequestConfig::default();

                // Initialize parameters from Swagger spec
                for param in &endpoint.parameters {
                    match param.location.as_str() {
                        "path" => {
                            // Path params: initialize with default or empty
                            if let Some(schema) = &param.schema {
                                if let Some(default) = &schema.default {
                                    let default_str = json_value_to_string(default);
                                    config.path_params.insert(param.name.clone(), default_str);
                                }
                            }
                            // Always insert path params (even if empty) so they show in UI
                            config
                                .path_params
                                .entry(param.name.clone())
                                .or_insert_with(String::new);
                        }
                        "query" => {
                            // Query params: initialize with default or empty
                            if let Some(schema) = &param.schema {
                                if let Some(default) = &schema.default {
                                    let default_str = json_value_to_string(default);
                                    config.query_params.insert(param.name.clone(), default_str);
                                }
                            }
                            // Always insert query params (even if empty) so they show in UI
                            config
                                .query_params
                                .entry(param.name.clone())
                                .or_insert_with(String::new);
                        }
                        _ => {
                            // Ignore other param types for now (header, cookie, etc.)
                        }
                    }
                }

                config
            })
    }

    /// Get the active endpoints list (filtered or full)
    pub fn active_endpoints(&self) -> &[ApiEndpoint] {
        if self.search_query.is_empty() {
            &self.endpoints
        } else {
            &self.filtered_endpoints
        }
    }

    /// Get the active grouped endpoints (filtered or full)
    pub fn active_grouped_endpoints(&self) -> &HashMap<String, Vec<ApiEndpoint>> {
        if self.search_query.is_empty() {
            &self.grouped_endpoints
        } else {
            &self.filtered_grouped_endpoints
        }
    }

    /// Filter endpoints based on search query
    pub fn update_filtered_endpoints(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_endpoints.clear();
            self.filtered_grouped_endpoints.clear();
            return;
        }

        let query = self.search_query.to_lowercase();

        // Filter endpoints by path, method, summary, or tags
        self.filtered_endpoints = self
            .endpoints
            .iter()
            .filter(|ep| {
                ep.path.to_lowercase().contains(&query)
                    || ep.method.to_lowercase().contains(&query)
                    || ep
                        .summary
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query))
                        .unwrap_or(false)
                    || ep.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();

        // Rebuild grouped endpoints from filtered list
        self.filtered_grouped_endpoints.clear();
        for endpoint in &self.filtered_endpoints {
            for tag in &endpoint.tags {
                self.filtered_grouped_endpoints
                    .entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(endpoint.clone());
            }

            // Handle endpoints without tags
            if endpoint.tags.is_empty() {
                self.filtered_grouped_endpoints
                    .entry("Other".to_string())
                    .or_insert_with(Vec::new)
                    .push(endpoint.clone());
            }
        }
    }
}

fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

/// Helper function to count visible items in current view mode
pub fn count_visible_items(state: &AppState) -> usize {
    match state.view_mode {
        ViewMode::Flat => state.active_endpoints().len(),
        ViewMode::Grouped => {
            let mut count = 0;
            let grouped = state.active_grouped_endpoints();
            let mut group_names: Vec<&String> = grouped.keys().collect();
            group_names.sort();

            for group_name in group_names {
                count += 1; // Group header
                if state.expanded_groups.contains(group_name) {
                    let endpoints = &grouped[group_name];
                    count += endpoints.len();
                }
            }
            count
        }
    }
}
