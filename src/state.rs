use crate::editor::BodyEditor;
use crate::types::{
    ApiEndpoint, ApiResponse, AuthState, DetailTab, InputMode, LoadingState, PanelFocus,
    RenderItem, RequestConfig, RequestEditMode, UrlInputField, ViewMode,
};
use std::collections::{HashMap, HashSet};

/// Data loaded from backend
#[derive(Debug, Clone)]
pub struct DataState {
    pub endpoints: Vec<ApiEndpoint>,
    pub grouped_endpoints: HashMap<String, Vec<ApiEndpoint>>,
    pub loading_state: LoadingState,
    pub retry_count: u32,
}

/// UI display and navigation state
#[derive(Debug, Clone)]
pub struct UiState {
    pub view_mode: ViewMode,
    pub expanded_groups: HashSet<String>,
    pub panel_focus: PanelFocus,
    pub active_detail_tab: DetailTab,
    pub selected_param_index: usize,
    pub body_section_expanded: bool,
}

/// Modal/form input state
#[derive(Debug, Clone)]
pub struct InputState {
    pub mode: InputMode,
    pub token_input: String,
    pub url_input: String,
    pub base_url_input: String,
    pub active_url_field: UrlInputField,
    pub body_editor: BodyEditor,
    pub body_validation_error: Option<String>,
}

/// HTTP request and authentication state
#[derive(Debug, Clone)]
pub struct RequestState {
    pub auth: AuthState,
    pub executing_endpoint: Option<String>,
    pub current_response: Option<ApiResponse>,
    pub configs: HashMap<String, RequestConfig>,
    pub edit_mode: RequestEditMode,
    pub param_edit_buffer: String,
}

/// Search and filtering state
#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub filtered_endpoints: Vec<ApiEndpoint>,
    pub filtered_grouped_endpoints: HashMap<String, Vec<ApiEndpoint>>,
}

/// Main application state - composed of logical sub-states
#[derive(Debug, Clone)]
pub struct AppState {
    pub data: DataState,
    pub ui: UiState,
    pub input: InputState,
    pub request: RequestState,
    pub search: SearchState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            data: DataState {
                endpoints: Vec::new(),
                grouped_endpoints: HashMap::new(),
                loading_state: LoadingState::Idle,
                retry_count: 0,
            },
            ui: UiState {
                view_mode: ViewMode::Grouped,
                expanded_groups: HashSet::new(),
                panel_focus: PanelFocus::EndpointsList,
                active_detail_tab: DetailTab::Endpoint,
                selected_param_index: 0,
                body_section_expanded: true,
            },
            input: InputState {
                mode: InputMode::Normal,
                token_input: String::new(),
                url_input: String::new(),
                base_url_input: String::new(),
                active_url_field: UrlInputField::SwaggerUrl,
                body_editor: BodyEditor::new(),
                body_validation_error: None,
            },
            request: RequestState {
                auth: AuthState::new(),
                executing_endpoint: None,
                current_response: None,
                configs: HashMap::new(),
                edit_mode: RequestEditMode::Viewing,
                param_edit_buffer: String::new(),
            },
            search: SearchState {
                query: String::new(),
                filtered_endpoints: Vec::new(),
                filtered_grouped_endpoints: HashMap::new(),
            },
        }
    }
}

impl AppState {
    /// Compute render items for grouped view on-demand
    /// This builds the flattened list of group headers and endpoints
    pub fn get_render_items(&self) -> Vec<RenderItem> {
        let mut render_items = Vec::new();
        let grouped = self.active_grouped_endpoints();
        let mut group_names: Vec<&String> = grouped.keys().collect();
        group_names.sort();

        for group_name in group_names {
            let group_endpoints = &grouped[group_name];
            let is_expanded = self.ui.expanded_groups.contains(group_name);

            render_items.push(RenderItem::GroupHeader {
                name: group_name.clone(),
                count: group_endpoints.len(),
                expanded: is_expanded,
            });

            if is_expanded {
                for endpoint in group_endpoints {
                    render_items.push(RenderItem::Endpoint {
                        endpoint: endpoint.clone(),
                    });
                }
            }
        }

        render_items
    }

    /// Get the selected endpoint based on the current view mode and selected index
    pub fn get_selected_endpoint(&self, selected_index: usize) -> Option<ApiEndpoint> {
        match self.ui.view_mode {
            ViewMode::Flat => self.data.endpoints.get(selected_index).cloned(),
            ViewMode::Grouped => {
                let render_items = self.get_render_items();
                render_items
                    .get(selected_index)
                    .and_then(|item| match item {
                        RenderItem::Endpoint { endpoint } => Some(endpoint.clone()),
                        RenderItem::GroupHeader { .. } => None,
                    })
            }
        }
    }

    /// Get or create request config for an endpoint, initializing with Swagger defaults
    pub fn get_or_create_request_config(&mut self, endpoint: &ApiEndpoint) -> &mut RequestConfig {
        self.request
            .configs
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
        if self.search.query.is_empty() {
            &self.data.endpoints
        } else {
            &self.search.filtered_endpoints
        }
    }

    /// Get the active grouped endpoints (filtered or full)
    pub fn active_grouped_endpoints(&self) -> &HashMap<String, Vec<ApiEndpoint>> {
        if self.search.query.is_empty() {
            &self.data.grouped_endpoints
        } else {
            &self.search.filtered_grouped_endpoints
        }
    }

    /// Get an endpoint by its path
    pub fn get_selected_endpoint_by_path(&self, path: &str) -> Option<&ApiEndpoint> {
        self.data.endpoints.iter().find(|ep| ep.path == path)
    }

    /// Get or create request config by endpoint path
    pub fn get_or_create_request_config_by_path(&mut self, path: &str) -> &mut RequestConfig {
        self.request
            .configs
            .entry(path.to_string())
            .or_insert(RequestConfig::default())
    }

    /// Filter endpoints based on search query
    pub fn update_filtered_endpoints(&mut self) {
        if self.search.query.is_empty() {
            self.search.filtered_endpoints.clear();
            self.search.filtered_grouped_endpoints.clear();
            return;
        }

        let query = self.search.query.to_lowercase();

        // Filter endpoints by path, method, summary, or tags
        self.search.filtered_endpoints = self
            .data
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
                    || ep
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();

        // Rebuild grouped endpoints from filtered list
        self.search.filtered_grouped_endpoints.clear();
        for endpoint in &self.search.filtered_endpoints {
            for tag in &endpoint.tags {
                self.search
                    .filtered_grouped_endpoints
                    .entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(endpoint.clone());
            }

            // Handle endpoints without tags
            if endpoint.tags.is_empty() {
                self.search
                    .filtered_grouped_endpoints
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
    match state.ui.view_mode {
        ViewMode::Flat => state.active_endpoints().len(),
        ViewMode::Grouped => {
            let mut count = 0;
            let grouped = state.active_grouped_endpoints();
            let mut group_names: Vec<&String> = grouped.keys().collect();
            group_names.sort();

            for group_name in group_names {
                count += 1; // Group header
                if state.ui.expanded_groups.contains(group_name) {
                    let endpoints = &grouped[group_name];
                    count += endpoints.len();
                }
            }
            count
        }
    }
}
