#[cfg(test)]
use crate::editor::BodyEditor;
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
    EnterBodyInputMode,
    ExitBodyInputMode,
    EnterConfirmClearTokenMode,
    ExitConfirmClearTokenMode,
    SetActiveUrlField(UrlInputField),

    // Text input actions (for modals)
    AppendToUrlInput(String),
    AppendToBaseUrlInput(String),
    AppendToTokenInput(String),
    AppendToSearchQuery(String),
    AppendToBodyInput(String),
    ClearUrlInput,
    ClearBaseUrlInput,
    ClearTokenInput,
    ClearSearchQuery,
    ClearBodyInput,
    BackspaceUrlInput,
    BackspaceBaseUrlInput,
    BackspaceTokenInput,
    BackspaceSearchQuery,
    BackspaceBodyInput,
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

    // Body section actions
    ToggleBodySection,
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
            state.ui.panel_focus = panel;
        }
        AppAction::NavigateToTab(tab) => {
            state.ui.active_detail_tab = tab;
            // Reset response scroll when navigating to/from Response tab
            state.ui.response_scroll = 0;
            state.ui.response_selected_line = 0;
        }
        AppAction::NavigateTabForward => {
            use DetailTab::*;
            match (&state.ui.panel_focus, &state.ui.active_detail_tab) {
                (PanelFocus::EndpointsList, _) => {
                    // Keep the current tab when switching to Details panel
                    state.ui.panel_focus = PanelFocus::Details;
                }
                (PanelFocus::Details, Endpoint) => {
                    state.ui.active_detail_tab = Request;
                    state.ui.selected_param_index = 0;
                }
                (PanelFocus::Details, Request) => {
                    state.ui.active_detail_tab = Headers;
                }
                (PanelFocus::Details, Headers) => {
                    state.ui.active_detail_tab = Response;
                    state.ui.response_scroll = 0;
                    state.ui.response_selected_line = 0;
                }
                (PanelFocus::Details, Response) => {
                    state.ui.panel_focus = PanelFocus::EndpointsList;
                    state.ui.active_detail_tab = Endpoint;
                }
            }
        }
        AppAction::NavigateTabBackward => {
            use DetailTab::*;
            match (&state.ui.panel_focus, &state.ui.active_detail_tab) {
                (PanelFocus::EndpointsList, _) => {
                    // Keep the current tab when switching to Details panel
                    state.ui.panel_focus = PanelFocus::Details;
                }
                (PanelFocus::Details, Request) => {
                    state.ui.active_detail_tab = Endpoint;
                    state.ui.selected_param_index = 0;
                }
                (PanelFocus::Details, Response) => {
                    state.ui.active_detail_tab = Headers;
                    state.ui.response_scroll = 0;
                    state.ui.response_selected_line = 0;
                }
                (PanelFocus::Details, Headers) => {
                    state.ui.active_detail_tab = Request;
                    state.ui.selected_param_index = 0;
                }
                (PanelFocus::Details, Endpoint) => {
                    state.ui.panel_focus = PanelFocus::EndpointsList;
                }
            }
        }
        AppAction::NavigateParamUp => {
            state.ui.selected_param_index = state.ui.selected_param_index.saturating_sub(1);
        }
        AppAction::NavigateParamDown => {
            state.ui.selected_param_index = state.ui.selected_param_index.saturating_add(1);
        }

        // View mode
        AppAction::ToggleViewMode => {
            use crate::types::ViewMode;
            state.ui.view_mode = match state.ui.view_mode {
                ViewMode::Flat => ViewMode::Grouped,
                ViewMode::Grouped => ViewMode::Flat,
            };
        }
        AppAction::ToggleGroupExpanded(group_name) => {
            if state.ui.expanded_groups.contains(&group_name) {
                state.ui.expanded_groups.remove(&group_name);
            } else {
                state.ui.expanded_groups.insert(group_name);
            }
        }

        // Input modes
        AppAction::EnterUrlInputMode {
            swagger_url,
            base_url,
        } => {
            state.input.mode = InputMode::EnteringUrl;
            state.input.url_input = swagger_url.unwrap_or_default();
            state.input.base_url_input = base_url.unwrap_or_default();
            state.input.active_url_field = UrlInputField::SwaggerUrl;
        }
        AppAction::ExitUrlInputMode => {
            state.input.mode = InputMode::Normal;
            state.input.url_input.clear();
            state.input.base_url_input.clear();
        }
        AppAction::EnterTokenInputMode => {
            state.input.mode = InputMode::EnteringToken;
            state.input.token_input.clear();
        }
        AppAction::ExitTokenInputMode => {
            state.input.mode = InputMode::Normal;
            state.input.token_input.clear();
        }
        AppAction::EnterSearchMode => {
            state.input.mode = InputMode::Searching;
            if state.search.query.is_empty() {
                state.search.query.clear();
            }
        }
        AppAction::ExitSearchMode => {
            state.input.mode = InputMode::Normal;
        }
        AppAction::EnterBodyInputMode => {
            state.input.mode = InputMode::EnteringBody;
            // Body input is pre-populated by caller
        }
        AppAction::ExitBodyInputMode => {
            state.input.mode = InputMode::Normal;
            state.input.body_editor.clear();
        }
        AppAction::EnterConfirmClearTokenMode => {
            state.input.mode = InputMode::ConfirmClearToken;
        }
        AppAction::ExitConfirmClearTokenMode => {
            state.input.mode = InputMode::Normal;
        }
        AppAction::SetActiveUrlField(field) => {
            state.input.active_url_field = field;
        }

        // Text input for modals
        AppAction::AppendToUrlInput(text) => {
            state.input.url_input.push_str(&text);
        }
        AppAction::AppendToBaseUrlInput(text) => {
            state.input.base_url_input.push_str(&text);
        }
        AppAction::AppendToTokenInput(text) => {
            state.input.token_input.push_str(&text);
        }
        AppAction::AppendToSearchQuery(text) => {
            state.search.query.push_str(&text);
        }
        AppAction::AppendToBodyInput(text) => {
            state.input.body_editor.insert_str(&text);
        }
        AppAction::ClearUrlInput => {
            state.input.url_input.clear();
        }
        AppAction::ClearBaseUrlInput => {
            state.input.base_url_input.clear();
        }
        AppAction::ClearTokenInput => {
            state.input.token_input.clear();
        }
        AppAction::ClearSearchQuery => {
            state.search.query.clear();
        }
        AppAction::ClearBodyInput => {
            state.input.body_editor.clear();
        }
        AppAction::BackspaceUrlInput => {
            state.input.url_input.pop();
        }
        AppAction::BackspaceBaseUrlInput => {
            state.input.base_url_input.pop();
        }
        AppAction::BackspaceTokenInput => {
            state.input.token_input.pop();
        }
        AppAction::BackspaceSearchQuery => {
            state.search.query.pop();
        }
        AppAction::BackspaceBodyInput => {
            state.input.body_editor.delete_char_before_cursor();
        }
        AppAction::DeleteWordUrlInput => {
            delete_word(&mut state.input.url_input);
        }
        AppAction::DeleteWordBaseUrlInput => {
            delete_word(&mut state.input.base_url_input);
        }
        AppAction::DeleteWordTokenInput => {
            delete_word(&mut state.input.token_input);
        }

        // Parameter editing
        AppAction::StartEditingParameter {
            param_name,
            endpoint_path,
        } => {
            state.request.edit_mode = RequestEditMode::Editing(param_name.clone());
            // Initialize buffer with current value if it exists
            if let Some(config) = state.request.configs.get(&endpoint_path) {
                if let Some(value) = config.path_params.get(&param_name) {
                    state.request.param_edit_buffer = value.clone();
                } else if let Some(value) = config.query_params.get(&param_name) {
                    state.request.param_edit_buffer = value.clone();
                } else {
                    state.request.param_edit_buffer.clear();
                }
            } else {
                state.request.param_edit_buffer.clear();
            }
        }
        AppAction::AppendToParamBuffer(text) => {
            state.request.param_edit_buffer.push_str(&text);
        }
        AppAction::BackspaceParamBuffer => {
            state.request.param_edit_buffer.pop();
        }
        AppAction::ClearParamBuffer => {
            state.request.param_edit_buffer.clear();
        }
        AppAction::ConfirmParameterEdit { endpoint_path } => {
            if let RequestEditMode::Editing(param_name) = &state.request.edit_mode {
                // Clone values we need before borrowing mutably
                let buffer_value = state.request.param_edit_buffer.clone();
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
            state.request.edit_mode = RequestEditMode::Viewing;
            state.request.param_edit_buffer.clear();
        }
        AppAction::CancelParameterEdit => {
            state.request.edit_mode = RequestEditMode::Viewing;
            state.request.param_edit_buffer.clear();
        }

        // Authentication
        AppAction::SetAuthToken(token) => {
            state.request.auth.set_token(token);
        }
        AppAction::ClearAuthToken => {
            state.request.auth.clear_token();
        }

        // Response
        AppAction::SetErrorResponse(error_msg) => {
            state.request.current_response = Some(crate::types::ApiResponse::error(error_msg));
        }
        AppAction::ClearResponse => {
            state.request.current_response = None;
        }

        // State resets
        AppAction::ResetParamIndex => {
            state.ui.selected_param_index = 0;
        }

        // Body section
        AppAction::ToggleBodySection => {
            state.ui.body_section_expanded = !state.ui.body_section_expanded;
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
        use crate::state::{DataState, InputState, RequestState, SearchState, UiState};

        AppState {
            data: DataState {
                endpoints: vec![],
                grouped_endpoints: HashMap::new(),
                loading_state: LoadingState::Idle,
                retry_count: 0,
            },
            ui: UiState {
                view_mode: ViewMode::Flat,
                expanded_groups: HashSet::new(),
                panel_focus: PanelFocus::EndpointsList,
                active_detail_tab: DetailTab::Endpoint,
                selected_param_index: 0,
                body_section_expanded: true,
                response_scroll: 0,
                response_selected_line: 0,
                yank_flash: false,
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

    #[test]
    fn test_navigate_to_panel() {
        let mut state = create_test_state();
        assert_eq!(state.ui.panel_focus, PanelFocus::EndpointsList);

        apply_action(AppAction::NavigateToPanel(PanelFocus::Details), &mut state);
        assert_eq!(state.ui.panel_focus, PanelFocus::Details);
    }

    #[test]
    fn test_navigate_tab_forward() {
        let mut state = create_test_state();
        state.ui.panel_focus = PanelFocus::Details;
        state.ui.active_detail_tab = DetailTab::Endpoint;

        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Request);

        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Headers);

        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Response);

        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.ui.panel_focus, PanelFocus::EndpointsList);
    }

    #[test]
    fn test_navigate_tab_backward() {
        let mut state = create_test_state();
        state.ui.panel_focus = PanelFocus::Details;
        state.ui.active_detail_tab = DetailTab::Response;

        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Headers);

        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Request);

        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Endpoint);

        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.ui.panel_focus, PanelFocus::EndpointsList);
    }

    #[test]
    fn test_tab_navigation_preserves_active_tab() {
        let mut state = create_test_state();

        // Start on Request tab in Details panel
        state.ui.panel_focus = PanelFocus::Details;
        state.ui.active_detail_tab = DetailTab::Request;

        // Navigate to EndpointsList
        apply_action(
            AppAction::NavigateToPanel(PanelFocus::EndpointsList),
            &mut state,
        );
        assert_eq!(state.ui.panel_focus, PanelFocus::EndpointsList);

        // Tab back to Details - should stay on Request tab
        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.ui.panel_focus, PanelFocus::Details);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Request);

        // Switch to Headers tab
        apply_action(AppAction::NavigateTabForward, &mut state);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Headers);

        // Navigate to EndpointsList with '1'
        apply_action(
            AppAction::NavigateToPanel(PanelFocus::EndpointsList),
            &mut state,
        );

        // Shift+Tab back to Details - should stay on Headers tab
        apply_action(AppAction::NavigateTabBackward, &mut state);
        assert_eq!(state.ui.panel_focus, PanelFocus::Details);
        assert_eq!(state.ui.active_detail_tab, DetailTab::Headers);
    }

    #[test]
    fn test_toggle_view_mode() {
        let mut state = create_test_state();
        assert_eq!(state.ui.view_mode, ViewMode::Flat);

        apply_action(AppAction::ToggleViewMode, &mut state);
        assert_eq!(state.ui.view_mode, ViewMode::Grouped);

        apply_action(AppAction::ToggleViewMode, &mut state);
        assert_eq!(state.ui.view_mode, ViewMode::Flat);
    }

    #[test]
    fn test_toggle_group_expanded() {
        let mut state = create_test_state();
        assert!(state.ui.expanded_groups.is_empty());

        apply_action(
            AppAction::ToggleGroupExpanded("Users".to_string()),
            &mut state,
        );
        assert!(state.ui.expanded_groups.contains("Users"));
        assert_eq!(state.ui.expanded_groups.len(), 1);

        apply_action(
            AppAction::ToggleGroupExpanded("Users".to_string()),
            &mut state,
        );
        assert!(state.ui.expanded_groups.is_empty());
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

        assert_eq!(state.input.mode, InputMode::EnteringUrl);
        assert_eq!(state.input.url_input, "http://localhost:5000/swagger.json");
        assert_eq!(state.input.base_url_input, "http://localhost:5000");
    }

    #[test]
    fn test_text_input_actions() {
        let mut state = create_test_state();

        apply_action(
            AppAction::AppendToUrlInput("http://".to_string()),
            &mut state,
        );
        assert_eq!(state.input.url_input, "http://");

        apply_action(
            AppAction::AppendToUrlInput("localhost".to_string()),
            &mut state,
        );
        assert_eq!(state.input.url_input, "http://localhost");

        apply_action(AppAction::BackspaceUrlInput, &mut state);
        assert_eq!(state.input.url_input, "http://localhos");

        apply_action(AppAction::ClearUrlInput, &mut state);
        assert_eq!(state.input.url_input, "");
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
        assert!(!state.request.auth.is_authenticated());

        apply_action(
            AppAction::SetAuthToken("my-secret-token".to_string()),
            &mut state,
        );
        assert!(state.request.auth.is_authenticated());

        apply_action(AppAction::ClearAuthToken, &mut state);
        assert!(!state.request.auth.is_authenticated());
    }

    #[test]
    fn test_parameter_buffer_actions() {
        let mut state = create_test_state();

        apply_action(
            AppAction::AppendToParamBuffer("123".to_string()),
            &mut state,
        );
        assert_eq!(state.request.param_edit_buffer, "123");

        apply_action(AppAction::BackspaceParamBuffer, &mut state);
        assert_eq!(state.request.param_edit_buffer, "12");

        apply_action(AppAction::ClearParamBuffer, &mut state);
        assert_eq!(state.request.param_edit_buffer, "");
    }

    #[test]
    fn test_cancel_parameter_edit() {
        let mut state = create_test_state();
        state.request.edit_mode = RequestEditMode::Editing("id".to_string());
        state.request.param_edit_buffer = "test value".to_string();

        apply_action(AppAction::CancelParameterEdit, &mut state);
        assert_eq!(state.request.edit_mode, RequestEditMode::Viewing);
        assert_eq!(state.request.param_edit_buffer, "");
    }

    #[test]
    fn test_navigate_param_up_down() {
        let mut state = create_test_state();
        state.ui.selected_param_index = 5;

        apply_action(AppAction::NavigateParamUp, &mut state);
        assert_eq!(state.ui.selected_param_index, 4);

        apply_action(AppAction::NavigateParamDown, &mut state);
        assert_eq!(state.ui.selected_param_index, 5);

        // Test saturation at zero
        state.ui.selected_param_index = 0;
        apply_action(AppAction::NavigateParamUp, &mut state);
        assert_eq!(state.ui.selected_param_index, 0);
    }

    #[test]
    fn test_search_actions() {
        let mut state = create_test_state();

        apply_action(AppAction::EnterSearchMode, &mut state);
        assert_eq!(state.input.mode, InputMode::Searching);

        apply_action(
            AppAction::AppendToSearchQuery("user".to_string()),
            &mut state,
        );
        assert_eq!(state.search.query, "user");

        apply_action(AppAction::BackspaceSearchQuery, &mut state);
        assert_eq!(state.search.query, "use");

        apply_action(AppAction::ClearSearchQuery, &mut state);
        assert_eq!(state.search.query, "");

        apply_action(AppAction::ExitSearchMode, &mut state);
        assert_eq!(state.input.mode, InputMode::Normal);
    }
}
