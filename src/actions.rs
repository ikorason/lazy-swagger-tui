use crate::state::AppState;
use crate::types::{DetailTab, InputMode, PanelFocus, RequestEditMode, UrlInputField};

/// Represents all possible state-changing actions in the application
/// This pattern separates input handling from state mutations, making the code
/// more testable and enabling future features like undo/redo
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Some actions defined for future use
pub enum AppAction {
    // Navigation actions
    NavigateUp,
    NavigateDown,
    NavigateToPanel(PanelFocus),
    NavigateToTab(DetailTab),
    NavigateTabForward,
    NavigateTabBackward,
    NavigateParamUp,
    NavigateParamDown,

    // Scrolling actions
    ScrollUp,
    ScrollDown,

    // View mode actions
    ToggleViewMode,
    ToggleGroupExpanded(String), // Toggle expand/collapse for a group

    // Input mode actions
    EnterUrlInputMode {
        swagger_url: Option<String>,
        base_url: Option<String>,
    },
    ExitUrlInputMode,
    EnterTokenInputMode,
    ExitTokenInputMode,
    EnterSearchMode,
    ExitSearchMode,
    EnterConfirmClearTokenMode,
    ExitConfirmClearTokenMode,
    SetActiveUrlField(UrlInputField),

    // Text input actions (for modals)
    AppendToUrlInput(String),
    AppendToBaseUrlInput(String),
    AppendToTokenInput(String),
    AppendToSearchQuery(String),
    ClearUrlInput,
    ClearBaseUrlInput,
    ClearTokenInput,
    ClearSearchQuery,
    BackspaceUrlInput,
    BackspaceBaseUrlInput,
    BackspaceTokenInput,
    BackspaceSearchQuery,
    DeleteWordUrlInput,
    DeleteWordBaseUrlInput,
    DeleteWordTokenInput,

    // Parameter editing actions
    StartEditingParameter {
        param_name: String,
        endpoint_path: String,
    },
    AppendToParamBuffer(String),
    BackspaceParamBuffer,
    ClearParamBuffer,
    ConfirmParameterEdit {
        endpoint_path: String,
    },
    CancelParameterEdit,

    // Authentication actions
    SetAuthToken(String),
    ClearAuthToken,

    // Response actions
    SetErrorResponse(String),
    ClearResponse,

    // State reset actions
    ResetParamIndex,
    ResetResponseScroll,
    ResetHeadersScroll,
}

/// Apply an action to the application state
/// This is a pure state transformation function that mutates AppState based on the action
/// All state mutations should go through this function to maintain consistency
pub fn apply_action(action: AppAction, state: &mut AppState) {
    match action {
        // Navigation
        AppAction::NavigateUp => {
            // This is handled in events.rs with list_state - not pure state
        }
        AppAction::NavigateDown => {
            // This is handled in events.rs with list_state - not pure state
        }
        AppAction::NavigateToPanel(panel) => {
            state.panel_focus = panel;
        }
        AppAction::NavigateToTab(tab) => {
            state.active_detail_tab = tab;
        }
        AppAction::NavigateTabForward => {
            use DetailTab::*;
            match (&state.panel_focus, &state.active_detail_tab) {
                (PanelFocus::EndpointsList, _) => {
                    state.panel_focus = PanelFocus::Details;
                    state.active_detail_tab = Endpoint;
                    state.selected_param_index = 0;
                }
                (PanelFocus::Details, Endpoint) => {
                    state.active_detail_tab = Request;
                    state.selected_param_index = 0;
                }
                (PanelFocus::Details, Request) => {
                    state.active_detail_tab = Headers;
                }
                (PanelFocus::Details, Headers) => {
                    state.active_detail_tab = Response;
                }
                (PanelFocus::Details, Response) => {
                    state.panel_focus = PanelFocus::EndpointsList;
                    state.active_detail_tab = Endpoint;
                }
            }
        }
        AppAction::NavigateTabBackward => {
            use DetailTab::*;
            match (&state.panel_focus, &state.active_detail_tab) {
                (PanelFocus::EndpointsList, _) => {
                    state.panel_focus = PanelFocus::Details;
                    state.active_detail_tab = Response;
                }
                (PanelFocus::Details, Request) => {
                    state.active_detail_tab = Endpoint;
                    state.selected_param_index = 0;
                }
                (PanelFocus::Details, Response) => {
                    state.active_detail_tab = Headers;
                }
                (PanelFocus::Details, Headers) => {
                    state.active_detail_tab = Request;
                    state.selected_param_index = 0;
                }
                (PanelFocus::Details, Endpoint) => {
                    state.panel_focus = PanelFocus::EndpointsList;
                }
            }
        }
        AppAction::NavigateParamUp => {
            state.selected_param_index = state.selected_param_index.saturating_sub(1);
        }
        AppAction::NavigateParamDown => {
            state.selected_param_index = state.selected_param_index.saturating_add(1);
        }

        // Scrolling
        AppAction::ScrollUp => {
            match state.active_detail_tab {
                DetailTab::Response => {
                    state.response_body_scroll = state.response_body_scroll.saturating_sub(5);
                }
                DetailTab::Headers => {
                    state.headers_scroll = state.headers_scroll.saturating_sub(5);
                }
                DetailTab::Endpoint | DetailTab::Request => {
                    // No scrolling for these tabs
                }
            }
        }
        AppAction::ScrollDown => {
            match state.active_detail_tab {
                DetailTab::Response => {
                    state.response_body_scroll = state.response_body_scroll.saturating_add(5);
                }
                DetailTab::Headers => {
                    state.headers_scroll = state.headers_scroll.saturating_add(5);
                }
                DetailTab::Endpoint | DetailTab::Request => {
                    // No scrolling for these tabs
                }
            }
        }

        // View mode
        AppAction::ToggleViewMode => {
            use crate::types::ViewMode;
            state.view_mode = match state.view_mode {
                ViewMode::Flat => ViewMode::Grouped,
                ViewMode::Grouped => ViewMode::Flat,
            };
        }
        AppAction::ToggleGroupExpanded(group_name) => {
            if state.expanded_groups.contains(&group_name) {
                state.expanded_groups.remove(&group_name);
            } else {
                state.expanded_groups.insert(group_name);
            }
        }

        // Input modes
        AppAction::EnterUrlInputMode {
            swagger_url,
            base_url,
        } => {
            state.input_mode = InputMode::EnteringUrl;
            state.url_input = swagger_url.unwrap_or_default();
            state.base_url_input = base_url.unwrap_or_default();
            state.active_url_field = UrlInputField::SwaggerUrl;
        }
        AppAction::ExitUrlInputMode => {
            state.input_mode = InputMode::Normal;
            state.url_input.clear();
            state.base_url_input.clear();
        }
        AppAction::EnterTokenInputMode => {
            state.input_mode = InputMode::EnteringToken;
            state.token_input.clear();
        }
        AppAction::ExitTokenInputMode => {
            state.input_mode = InputMode::Normal;
            state.token_input.clear();
        }
        AppAction::EnterSearchMode => {
            state.input_mode = InputMode::Searching;
            state.search_query.clear();
        }
        AppAction::ExitSearchMode => {
            state.input_mode = InputMode::Normal;
        }
        AppAction::EnterConfirmClearTokenMode => {
            state.input_mode = InputMode::ConfirmClearToken;
        }
        AppAction::ExitConfirmClearTokenMode => {
            state.input_mode = InputMode::Normal;
        }
        AppAction::SetActiveUrlField(field) => {
            state.active_url_field = field;
        }

        // Text input for modals
        AppAction::AppendToUrlInput(text) => {
            state.url_input.push_str(&text);
        }
        AppAction::AppendToBaseUrlInput(text) => {
            state.base_url_input.push_str(&text);
        }
        AppAction::AppendToTokenInput(text) => {
            state.token_input.push_str(&text);
        }
        AppAction::AppendToSearchQuery(text) => {
            state.search_query.push_str(&text);
        }
        AppAction::ClearUrlInput => {
            state.url_input.clear();
        }
        AppAction::ClearBaseUrlInput => {
            state.base_url_input.clear();
        }
        AppAction::ClearTokenInput => {
            state.token_input.clear();
        }
        AppAction::ClearSearchQuery => {
            state.search_query.clear();
        }
        AppAction::BackspaceUrlInput => {
            state.url_input.pop();
        }
        AppAction::BackspaceBaseUrlInput => {
            state.base_url_input.pop();
        }
        AppAction::BackspaceTokenInput => {
            state.token_input.pop();
        }
        AppAction::BackspaceSearchQuery => {
            state.search_query.pop();
        }
        AppAction::DeleteWordUrlInput => {
            delete_word(&mut state.url_input);
        }
        AppAction::DeleteWordBaseUrlInput => {
            delete_word(&mut state.base_url_input);
        }
        AppAction::DeleteWordTokenInput => {
            delete_word(&mut state.token_input);
        }

        // Parameter editing
        AppAction::StartEditingParameter {
            param_name,
            endpoint_path,
        } => {
            state.request_edit_mode = RequestEditMode::Editing(param_name.clone());
            // Initialize buffer with current value if it exists
            if let Some(config) = state.request_configs.get(&endpoint_path) {
                if let Some(value) = config.path_params.get(&param_name) {
                    state.param_edit_buffer = value.clone();
                } else if let Some(value) = config.query_params.get(&param_name) {
                    state.param_edit_buffer = value.clone();
                } else {
                    state.param_edit_buffer.clear();
                }
            } else {
                state.param_edit_buffer.clear();
            }
        }
        AppAction::AppendToParamBuffer(text) => {
            state.param_edit_buffer.push_str(&text);
        }
        AppAction::BackspaceParamBuffer => {
            state.param_edit_buffer.pop();
        }
        AppAction::ClearParamBuffer => {
            state.param_edit_buffer.clear();
        }
        AppAction::ConfirmParameterEdit { endpoint_path } => {
            if let RequestEditMode::Editing(param_name) = &state.request_edit_mode {
                // Clone values we need before borrowing mutably
                let buffer_value = state.param_edit_buffer.clone();
                let param_name = param_name.clone();

                // Determine if this is a path or query param
                let is_path_param = state
                    .get_selected_endpoint_by_path(&endpoint_path)
                    .map(|endpoint| {
                        endpoint
                            .parameters
                            .iter()
                            .any(|p| &p.name == &param_name && p.location == "path")
                    })
                    .unwrap_or(false);

                // Get or create the config and insert the value
                let config = state.get_or_create_request_config_by_path(&endpoint_path);

                if is_path_param {
                    config.path_params.insert(param_name, buffer_value);
                } else {
                    config.query_params.insert(param_name, buffer_value);
                }
            }
            state.request_edit_mode = RequestEditMode::Viewing;
            state.param_edit_buffer.clear();
        }
        AppAction::CancelParameterEdit => {
            state.request_edit_mode = RequestEditMode::Viewing;
            state.param_edit_buffer.clear();
        }

        // Authentication
        AppAction::SetAuthToken(token) => {
            state.auth.set_token(token);
        }
        AppAction::ClearAuthToken => {
            state.auth.clear_token();
        }

        // Response
        AppAction::SetErrorResponse(error_msg) => {
            state.current_response = Some(crate::types::ApiResponse::error(error_msg));
        }
        AppAction::ClearResponse => {
            state.current_response = None;
        }

        // State resets
        AppAction::ResetParamIndex => {
            state.selected_param_index = 0;
        }
        AppAction::ResetResponseScroll => {
            state.response_body_scroll = 0;
        }
        AppAction::ResetHeadersScroll => {
            state.headers_scroll = 0;
        }
    }
}

/// Helper function to delete the last word from a string (Ctrl+W behavior)
fn delete_word(s: &mut String) {
    // Trim trailing whitespace first
    *s = s.trim_end().to_string();

    // Find last whitespace and truncate there
    if let Some(pos) = s.rfind(char::is_whitespace) {
        s.truncate(pos);
    } else {
        // No whitespace found, clear entire string
        s.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AuthState, LoadingState, ViewMode};
    use std::collections::{HashMap, HashSet};

    fn create_test_state() -> AppState {
        AppState {
            endpoints: vec![],
            grouped_endpoints: HashMap::new(),
            loading_state: LoadingState::Idle,
            retry_count: 0,
            view_mode: ViewMode::Flat,
            expanded_groups: HashSet::new(),
            panel_focus: PanelFocus::EndpointsList,
            active_detail_tab: DetailTab::Endpoint,
            selected_param_index: 0,
            input_mode: InputMode::Normal,
            url_input: String::new(),
            base_url_input: String::new(),
            active_url_field: UrlInputField::SwaggerUrl,
            token_input: String::new(),
            auth: AuthState::new(),
            executing_endpoint: None,
            current_response: None,
            request_edit_mode: RequestEditMode::Viewing,
            param_edit_buffer: String::new(),
            request_configs: HashMap::new(),
            response_body_scroll: 0,
            headers_scroll: 0,
            search_query: String::new(),
            filtered_endpoints: Vec::new(),
            filtered_grouped_endpoints: HashMap::new(),
        }
    }

    #[test]
    fn test_navigate_to_panel() {
        let mut state = create_test_state();
        assert_eq!(state.panel_focus, PanelFocus::EndpointsList);

        apply_action(AppAction::NavigateToPanel(PanelFocus::Details), &mut state);
        assert_eq!(state.panel_focus, PanelFocus::Details);
    }

    #[test]
    fn test_navigate_tab_forward() {
        let mut state = create_test_state();
        state.panel_focus = PanelFocus::Details;
        state.active_detail_tab = DetailTab::Endpoint;

        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.active_detail_tab, DetailTab::Request);

        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.active_detail_tab, DetailTab::Headers);

        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.active_detail_tab, DetailTab::Response);

        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.panel_focus, PanelFocus::EndpointsList);
    }

    #[test]
    fn test_navigate_tab_backward() {
        let mut state = create_test_state();
        state.panel_focus = PanelFocus::Details;
        state.active_detail_tab = DetailTab::Response;

        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.active_detail_tab, DetailTab::Headers);

        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.active_detail_tab, DetailTab::Request);

        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.active_detail_tab, DetailTab::Endpoint);

        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.panel_focus, PanelFocus::EndpointsList);
    }

    #[test]
    fn test_toggle_view_mode() {
        let mut state = create_test_state();
        assert_eq!(state.view_mode, ViewMode::Flat);

        apply_action(AppAction::ToggleViewMode, &mut state);
        assert_eq!(state.view_mode, ViewMode::Grouped);

        apply_action(AppAction::ToggleViewMode, &mut state);
        assert_eq!(state.view_mode, ViewMode::Flat);
    }

    #[test]
    fn test_toggle_group_expanded() {
        let mut state = create_test_state();
        assert!(state.expanded_groups.is_empty());

        apply_action(
            AppAction::ToggleGroupExpanded("Users".to_string()),
            &mut state,
        );
        assert!(state.expanded_groups.contains("Users"));
        assert_eq!(state.expanded_groups.len(), 1);

        apply_action(
            AppAction::ToggleGroupExpanded("Users".to_string()),
            &mut state,
        );
        assert!(state.expanded_groups.is_empty());
    }

    #[test]
    fn test_scroll_actions() {
        let mut state = create_test_state();
        state.panel_focus = PanelFocus::Details;
        state.active_detail_tab = DetailTab::Response;
        state.response_body_scroll = 10;

        apply_action(AppAction::ScrollDown, &mut state);
        assert_eq!(state.response_body_scroll, 15);

        apply_action(AppAction::ScrollUp, &mut state);
        assert_eq!(state.response_body_scroll, 10);

        apply_action(AppAction::ScrollUp, &mut state);
        assert_eq!(state.response_body_scroll, 5);
    }

    #[test]
    fn test_enter_url_input_mode() {
        let mut state = create_test_state();

        apply_action(
            AppAction::EnterUrlInputMode {
                swagger_url: Some("http://localhost:5000/swagger.json".to_string()),
                base_url: Some("http://localhost:5000".to_string()),
            },
            &mut state,
        );

        assert_eq!(state.input_mode, InputMode::EnteringUrl);
        assert_eq!(state.url_input, "http://localhost:5000/swagger.json");
        assert_eq!(state.base_url_input, "http://localhost:5000");
    }

    #[test]
    fn test_text_input_actions() {
        let mut state = create_test_state();

        apply_action(
            AppAction::AppendToUrlInput("http://".to_string()),
            &mut state,
        );
        assert_eq!(state.url_input, "http://");

        apply_action(
            AppAction::AppendToUrlInput("localhost".to_string()),
            &mut state,
        );
        assert_eq!(state.url_input, "http://localhost");

        apply_action(AppAction::BackspaceUrlInput, &mut state);
        assert_eq!(state.url_input, "http://localhos");

        apply_action(AppAction::ClearUrlInput, &mut state);
        assert_eq!(state.url_input, "");
    }

    #[test]
    fn test_delete_word() {
        let mut s = "hello world foo".to_string();
        delete_word(&mut s);
        assert_eq!(s, "hello world");

        delete_word(&mut s);
        assert_eq!(s, "hello");

        delete_word(&mut s);
        assert_eq!(s, "");

        delete_word(&mut s);
        assert_eq!(s, "");
    }

    #[test]
    fn test_delete_word_with_trailing_space() {
        let mut s = "hello world   ".to_string();
        delete_word(&mut s);
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_auth_token_actions() {
        let mut state = create_test_state();
        assert!(!state.auth.is_authenticated());

        apply_action(
            AppAction::SetAuthToken("my-secret-token".to_string()),
            &mut state,
        );
        assert!(state.auth.is_authenticated());

        apply_action(AppAction::ClearAuthToken, &mut state);
        assert!(!state.auth.is_authenticated());
    }

    #[test]
    fn test_parameter_buffer_actions() {
        let mut state = create_test_state();

        apply_action(
            AppAction::AppendToParamBuffer("123".to_string()),
            &mut state,
        );
        assert_eq!(state.param_edit_buffer, "123");

        apply_action(AppAction::BackspaceParamBuffer, &mut state);
        assert_eq!(state.param_edit_buffer, "12");

        apply_action(AppAction::ClearParamBuffer, &mut state);
        assert_eq!(state.param_edit_buffer, "");
    }

    #[test]
    fn test_cancel_parameter_edit() {
        let mut state = create_test_state();
        state.request_edit_mode = RequestEditMode::Editing("id".to_string());
        state.param_edit_buffer = "test value".to_string();

        apply_action(AppAction::CancelParameterEdit, &mut state);
        assert_eq!(state.request_edit_mode, RequestEditMode::Viewing);
        assert_eq!(state.param_edit_buffer, "");
    }

    #[test]
    fn test_navigate_param_up_down() {
        let mut state = create_test_state();
        state.selected_param_index = 5;

        apply_action(AppAction::NavigateParamUp, &mut state);
        assert_eq!(state.selected_param_index, 4);

        apply_action(AppAction::NavigateParamDown, &mut state);
        assert_eq!(state.selected_param_index, 5);

        // Test saturation at zero
        state.selected_param_index = 0;
        apply_action(AppAction::NavigateParamUp, &mut state);
        assert_eq!(state.selected_param_index, 0);
    }

    #[test]
    fn test_search_actions() {
        let mut state = create_test_state();

        apply_action(AppAction::EnterSearchMode, &mut state);
        assert_eq!(state.input_mode, InputMode::Searching);

        apply_action(
            AppAction::AppendToSearchQuery("user".to_string()),
            &mut state,
        );
        assert_eq!(state.search_query, "user");

        apply_action(AppAction::BackspaceSearchQuery, &mut state);
        assert_eq!(state.search_query, "use");

        apply_action(AppAction::ClearSearchQuery, &mut state);
        assert_eq!(state.search_query, "");

        apply_action(AppAction::ExitSearchMode, &mut state);
        assert_eq!(state.input_mode, InputMode::Normal);
    }
}
