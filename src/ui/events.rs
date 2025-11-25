use crate::config;
use crate::request::execute_request_background;
use crate::state::{AppState, count_visible_items};
use crate::types::{
    ApiEndpoint, ApiResponse, DetailTab, InputMode, PanelFocus, RenderItem, RequestConfig,
    RequestEditMode, UrlInputField, UrlSubmission, ViewMode,
};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::widgets::ListState;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct EventHandler {
    pub should_quit: bool,
    pub selected_index: usize,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            selected_index: 0,
        }
    }

    pub fn handle_events(
        &mut self,
        state: Arc<RwLock<AppState>>,
        list_state: &mut ListState,
        base_url: Option<String>,
        swagger_url: Option<String>,
    ) -> Result<(bool, Option<UrlSubmission>)> {
        let mut should_fetch = false;
        let mut url_submitted = None;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let input_mode = state.read().unwrap().input_mode.clone();

                match input_mode {
                    InputMode::EnteringUrl => {
                        url_submitted = self.handle_url_input(key, state.clone())?;
                    }

                    InputMode::EnteringToken => {
                        self.handle_token_input(key, state.clone())?;
                    }

                    InputMode::ConfirmClearToken => {
                        self.handle_clear_confirmation(key, state.clone())?;
                    }

                    InputMode::Searching => {
                        self.handle_search_input(key, state.clone(), list_state)?;
                    }

                    InputMode::Normal => match key.code {
                        // QUIT
                        KeyCode::Char('q') => {
                            // Don't quit if we're editing a parameter
                            let state_read = state.read().unwrap();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // We're editing - treat 'q' as character input
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push('q');
                            } else {
                                // Not editing - quit the app
                                self.should_quit = true;
                            }
                        }
                        // nav down
                        KeyCode::Char('j') => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            // If editing a parameter, treat as character input
                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push('j');
                            } else {
                                // Not editing - handle navigation
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        // Navigate down in endpoints list
                                        self.handle_down(state.clone(), list_state);
                                    }
                                    PanelFocus::Details => {
                                        // If on Request tab and in Viewing mode, navigate params
                                        if active_tab == DetailTab::Request {
                                            self.handle_request_param_down(state.clone());
                                        }
                                        // For other tabs, j/k do nothing (use Ctrl+d/u for scrolling)
                                    }
                                }
                            }
                        }
                        // nav up
                        KeyCode::Char('k') => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            // If editing a parameter, treat as character input
                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push('k');
                            } else {
                                // Not editing - handle navigation
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        // Navigate up in endpoints list
                                        self.handle_up(state.clone(), list_state);
                                    }
                                    PanelFocus::Details => {
                                        // If on Request tab and in Viewing mode, navigate params
                                        if active_tab == DetailTab::Request {
                                            self.handle_request_param_up(state.clone());
                                        }
                                        // For other tabs, j/k do nothing (use Ctrl+d/u for scrolling)
                                    }
                                }
                            }
                        }
                        // handle auth dialog
                        KeyCode::Char('a') => {
                            let state_read = state.read().unwrap();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // We're editing - treat 'a' as character input
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push('a');
                            } else {
                                // Not editing - auth dialog
                                self.handle_auth_dialog(state.clone());
                            }
                        }
                        // edit param
                        KeyCode::Char('e') => {
                            let state_read = state.read().unwrap();
                            let edit_mode = state_read.request_edit_mode.clone();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            drop(state_read);

                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // We're editing - treat 'e' as character input
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push('e');
                            } else {
                                // Only handle if on Details panel and Request tab
                                if panel == PanelFocus::Details && active_tab == DetailTab::Request
                                {
                                    self.handle_request_param_edit(state.clone());
                                }
                            }
                        }
                        // toggle view - list <-> grouped
                        KeyCode::Char('g') => {
                            let state_read = state.read().unwrap();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // We're editing - treat 'g' as character input
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push('g');
                            } else {
                                self.handle_toggle_view(state.clone(), list_state);
                            }
                        }
                        // config url
                        KeyCode::Char(',') => {
                            let state_read = state.read().unwrap();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // We're editing - treat ',' as character input
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push(',');
                            } else {
                                self.handle_url_dialog(
                                    state.clone(),
                                    swagger_url.clone(),
                                    base_url.clone(),
                                );
                            }
                        }
                        // search endpoints
                        KeyCode::Char('/') => {
                            let state_read = state.read().unwrap();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // We're editing - treat '/' as character input
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push('/');
                            } else {
                                self.handle_search_activate(state.clone());
                            }
                        }

                        // ctrl + modifiers
                        // retry
                        KeyCode::Char('r')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            should_fetch = self.handle_retry(state.clone());
                        }

                        // Ctrl+l: Clear search filter
                        KeyCode::Char('l')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            self.handle_search_clear(state.clone(), list_state);
                        }

                        // -- with modifiers
                        // Ctrl+u: Scroll up in focused section
                        KeyCode::Char('u')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            if panel == PanelFocus::Details {
                                let mut s = state.write().unwrap();
                                use crate::types::DetailTab;

                                match s.active_detail_tab {
                                    DetailTab::Response => {
                                        s.response_body_scroll =
                                            s.response_body_scroll.saturating_sub(5);
                                    }
                                    DetailTab::Headers => {
                                        s.headers_scroll = s.headers_scroll.saturating_sub(5);
                                    }
                                    DetailTab::Endpoint | DetailTab::Request => {
                                        // Endpoint tab doesn't scroll
                                    }
                                }
                            }
                        }
                        // Ctrl+d: Scroll down in focused section
                        KeyCode::Char('d')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            drop(state_read);

                            if panel == PanelFocus::Details {
                                let mut s = state.write().unwrap();

                                match s.active_detail_tab {
                                    DetailTab::Response => {
                                        s.response_body_scroll =
                                            s.response_body_scroll.saturating_add(5);
                                    }
                                    DetailTab::Headers => {
                                        s.headers_scroll = s.headers_scroll.saturating_add(5);
                                    }
                                    DetailTab::Endpoint | DetailTab::Request => {
                                        // Endpoint tab doesn't scroll
                                    }
                                }
                            }
                        }

                        // Special keys --
                        // tab navigation
                        KeyCode::Tab => {
                            let mut s = state.write().unwrap();
                            use crate::types::{DetailTab, PanelFocus};

                            match s.panel_focus {
                                PanelFocus::EndpointsList => {
                                    // Move from Endpoints panel to Details panel (Endpoint tab)
                                    s.panel_focus = PanelFocus::Details;
                                    s.active_detail_tab = DetailTab::Endpoint;
                                    s.selected_param_index = 0; // Reset to first param
                                }
                                PanelFocus::Details => {
                                    // Cycle through tabs in Details panel
                                    s.active_detail_tab = match s.active_detail_tab {
                                        DetailTab::Endpoint => {
                                            s.selected_param_index = 0; // Reset when entering Request tab
                                            DetailTab::Request
                                        }
                                        DetailTab::Request => DetailTab::Headers,
                                        DetailTab::Headers => DetailTab::Response,
                                        DetailTab::Response => {
                                            // Wrap back to Endpoints panel
                                            s.panel_focus = PanelFocus::EndpointsList;
                                            DetailTab::Endpoint // Reset to first tab for next time
                                        }
                                    };
                                }
                            }
                        }
                        // Shift+Tab (BackTab) - move left
                        KeyCode::BackTab => {
                            let mut s = state.write().unwrap();
                            use crate::types::{DetailTab, PanelFocus};

                            match s.panel_focus {
                                PanelFocus::EndpointsList => {
                                    // Move from Endpoints panel to Details panel (Response tab - rightmost)
                                    s.panel_focus = PanelFocus::Details;
                                    s.active_detail_tab = DetailTab::Response;
                                }
                                PanelFocus::Details => {
                                    // Cycle backwards through tabs in Details panel
                                    match s.active_detail_tab {
                                        DetailTab::Request => {
                                            s.selected_param_index = 0; // Reset when leaving Request tab
                                            s.active_detail_tab = DetailTab::Endpoint;
                                        }
                                        DetailTab::Response => {
                                            s.active_detail_tab = DetailTab::Headers;
                                        }
                                        DetailTab::Headers => {
                                            s.selected_param_index = 0; // Reset when entering Request tab
                                            s.active_detail_tab = DetailTab::Request;
                                        }
                                        DetailTab::Endpoint => {
                                            // Wrap back to Endpoints panel
                                            s.panel_focus = PanelFocus::EndpointsList;
                                            // Keep active_detail_tab as Endpoint - don't change it
                                        }
                                    }
                                }
                            }
                        }
                        // space  - execute & expand
                        KeyCode::Char(' ') => {
                            let state_read = state.read().unwrap();
                            let edit_mode = state_read.request_edit_mode.clone();
                            let panel = state_read.panel_focus.clone();
                            drop(state_read);

                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // We're editing - treat space as character input
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push(' ');
                            } else {
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        // Space executes request or expands group
                                        self.handle_enter(
                                            state.clone(),
                                            list_state,
                                            base_url.clone(),
                                        );
                                    }
                                    PanelFocus::Details => {
                                        // Space in Details panel: Execute current endpoint again
                                        let state_read = state.read().unwrap();

                                        let selected_endpoint = state_read.get_selected_endpoint(self.selected_index).cloned();

                                        if let Some(endpoint) = selected_endpoint {
                                            if let Some(base_url) = base_url.clone() {
                                                // Check if already executing
                                                if let Some(ref executing) =
                                                    state_read.executing_endpoint
                                                {
                                                    if executing == &endpoint.path {
                                                        log_debug("Request already in progress");
                                                        return Ok((false, None));
                                                    }
                                                }

                                                // Validate that all required path params are filled
                                                let config =
                                                    state_read.request_configs.get(&endpoint.path);
                                                if let Err(err_msg) =
                                                    can_execute_endpoint(&endpoint, config)
                                                {
                                                    log_debug(&format!(
                                                        "Cannot execute: {}",
                                                        err_msg
                                                    ));
                                                    drop(state_read);

                                                    // Store error in response so user can see it
                                                    let mut s = state.write().unwrap();
                                                    s.current_response = Some(ApiResponse::error(err_msg));
                                                    return Ok((false, None));
                                                }

                                                drop(state_read);
                                                execute_request_background(
                                                    state.clone(),
                                                    endpoint,
                                                    base_url,
                                                );
                                            } else {
                                                log_debug(
                                                    "Cannot execute: Base URL not configured",
                                                );
                                            }
                                        } else {
                                            log_debug(
                                                "No endpoint selected (group header or empty)",
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        // enter - param confirm
                        KeyCode::Enter => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            // ONLY handle if on Request tab and in Editing mode
                            if panel == PanelFocus::Details
                                && active_tab == DetailTab::Request
                                && matches!(edit_mode, RequestEditMode::Editing(_))
                            {
                                self.handle_request_param_confirm(state.clone());
                            }
                        }
                        // backspace - param edit
                        KeyCode::Backspace => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            // ONLY handle if on Request tab and in Editing mode
                            if panel == PanelFocus::Details
                                && active_tab == DetailTab::Request
                                && matches!(edit_mode, RequestEditMode::Editing(_))
                            {
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.pop();
                            }
                        }
                        // esc - cancel param edit
                        KeyCode::Esc => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            // ONLY handle if on Request tab and in Editing mode
                            if panel == PanelFocus::Details
                                && active_tab == DetailTab::Request
                                && matches!(edit_mode, RequestEditMode::Editing(_))
                            {
                                self.handle_request_param_cancel(state.clone());
                            }
                        }

                        // keep arrow keys for accessibility (optional)
                        KeyCode::Up => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            // Don't handle navigation during parameter editing
                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // Do nothing - let user type normally
                            } else {
                                use crate::types::PanelFocus;
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        self.handle_up(state.clone(), list_state);
                                    }
                                    PanelFocus::Details => {
                                        if active_tab == DetailTab::Request {
                                            self.handle_request_param_up(state.clone());
                                        }
                                    }
                                }
                            }
                        }

                        KeyCode::Down => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            // Don't handle navigation during parameter editing
                            if matches!(edit_mode, RequestEditMode::Editing(_)) {
                                // Do nothing - let user type normally
                            } else {
                                use crate::types::PanelFocus;
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        self.handle_down(state.clone(), list_state);
                                    }
                                    PanelFocus::Details => {
                                        if active_tab == DetailTab::Request {
                                            self.handle_request_param_down(state.clone());
                                        }
                                    }
                                }
                            }
                        }

                        KeyCode::Char(c)
                            if !key.modifiers.contains(KeyModifiers::CONTROL) && c != ' ' =>
                        {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            let active_tab = state_read.active_detail_tab.clone();
                            let edit_mode = state_read.request_edit_mode.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            if panel == PanelFocus::Details
                                && active_tab == DetailTab::Request
                                && matches!(edit_mode, RequestEditMode::Editing(_))
                            {
                                let mut s = state.write().unwrap();
                                s.param_edit_buffer.push(c);
                                log_debug(&format!(
                                    "Added char, buffer now: {}",
                                    s.param_edit_buffer
                                ));
                            } else {
                                log_debug("Conditions not met for character input");
                            }
                        }

                        _ => {}
                    },
                }
            }
        }
        Ok((should_fetch, url_submitted))
    }

    fn handle_url_dialog(
        &self,
        state: Arc<RwLock<AppState>>,
        swagger_url: Option<String>,
        base_url: Option<String>,
    ) {
        let mut s = state.write().unwrap();
        s.input_mode = InputMode::EnteringUrl;

        // pre-fill with current swagger URL if exists
        s.url_input = swagger_url.unwrap_or_default();
        s.base_url_input = base_url.unwrap_or_default();
        s.active_url_field = UrlInputField::SwaggerUrl;

        log_debug("Entering URL input mode");
    }

    fn handle_url_input(
        &self,
        key: crossterm::event::KeyEvent,
        state: Arc<RwLock<AppState>>,
    ) -> Result<Option<UrlSubmission>> {
        use crate::types::UrlInputField;
        use crossterm::event::KeyModifiers;

        match key.code {
            KeyCode::Tab => {
                // Switch between fields and auto-extract when leaving swagger field
                let mut s = state.write().unwrap();

                match s.active_url_field {
                    UrlInputField::SwaggerUrl => {
                        // Moving from Swagger to Base - only auto-extract if base is empty
                        if !s.url_input.is_empty() && s.base_url_input.is_empty() {
                            s.base_url_input = config::extract_base_url(&s.url_input);
                            log_debug(&format!("Auto-extracted base URL: {}", s.base_url_input));
                        } else if !s.base_url_input.is_empty() {
                            log_debug("Base URL already exists, not auto-extracting");
                        }
                        s.active_url_field = UrlInputField::BaseUrl;
                    }
                    UrlInputField::BaseUrl => {
                        s.active_url_field = UrlInputField::SwaggerUrl;
                    }
                }
            }

            KeyCode::Enter => {
                let mut s = state.write().unwrap();
                let swagger_url = s.url_input.trim().to_string();
                let base_url = s.base_url_input.trim().to_string();

                if !swagger_url.is_empty() {
                    // Validate both URLs
                    match config::validate_url(&swagger_url) {
                        Ok(_) => {
                            // Also validate base URL if provided
                            if !base_url.is_empty() {
                                if let Err(e) = config::validate_url(&base_url) {
                                    log_debug(&format!("Invalid base URL: {}", e));
                                    // Keep modal open
                                    return Ok(None);
                                }
                            }

                            s.input_mode = InputMode::Normal;

                            let submission = crate::types::UrlSubmission {
                                swagger_url: swagger_url.clone(),
                                base_url: if base_url.is_empty() {
                                    None
                                } else {
                                    Some(base_url.clone())
                                },
                            };

                            s.url_input.clear();
                            s.base_url_input.clear();
                            s.active_url_field = UrlInputField::SwaggerUrl;

                            log_debug(&format!(
                                "URLs submitted - Swagger: {}, Base: {:?}",
                                submission.swagger_url, submission.base_url
                            ));

                            return Ok(Some(submission));
                        }
                        Err(e) => {
                            log_debug(&format!("Invalid swagger URL: {}", e));
                        }
                    }
                } else {
                    log_debug("Empty swagger URL, not submitting");
                }
            }

            KeyCode::Esc => {
                let mut s = state.write().unwrap();
                s.input_mode = InputMode::Normal;
                s.url_input.clear();
                s.base_url_input.clear();
                s.active_url_field = UrlInputField::SwaggerUrl;
                log_debug("URL input cancelled");
            }

            KeyCode::Backspace => {
                let mut s = state.write().unwrap();
                match s.active_url_field {
                    UrlInputField::SwaggerUrl => {
                        s.url_input.pop();
                    }
                    UrlInputField::BaseUrl => {
                        s.base_url_input.pop();
                    }
                }
            }

            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+U: Clear entire line
                let mut s = state.write().unwrap();
                match s.active_url_field {
                    UrlInputField::SwaggerUrl => {
                        s.url_input.clear();
                        log_debug("Cleared swagger URL input");
                    }
                    UrlInputField::BaseUrl => {
                        s.base_url_input.clear();
                        log_debug("Cleared base URL input");
                    }
                }
            }

            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+W: Delete word backwards
                let mut s = state.write().unwrap();
                let input = match s.active_url_field {
                    UrlInputField::SwaggerUrl => &mut s.url_input,
                    UrlInputField::BaseUrl => &mut s.base_url_input,
                };

                // Find last word boundary (space, slash, colon, dot)
                if let Some(pos) =
                    input.rfind(|c: char| c == ' ' || c == '/' || c == ':' || c == '.')
                {
                    input.truncate(pos);
                } else {
                    input.clear();
                }
            }

            KeyCode::Char(c) => {
                // Collect this character and any pending characters (for paste support)
                let mut chars = vec![c];

                // Drain any immediately available character events
                loop {
                    match event::poll(std::time::Duration::from_millis(0)) {
                        Ok(true) => {
                            if let Ok(Event::Key(next_key)) = event::read() {
                                match next_key.code {
                                    KeyCode::Char(next_c)
                                        if !next_key.modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
                                        chars.push(next_c);
                                    }
                                    _ => {
                                        // Non-character or control key, stop batching
                                        break;
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }

                // Log before consuming chars
                let char_count = chars.len();

                // Add all batched characters at once
                let mut s = state.write().unwrap();
                for ch in chars {
                    match s.active_url_field {
                        UrlInputField::SwaggerUrl => {
                            s.url_input.push(ch);
                        }
                        UrlInputField::BaseUrl => {
                            s.base_url_input.push(ch);
                        }
                    }
                }

                if char_count > 1 {
                    log_debug(&format!("Batched {} characters in URL input", char_count));
                }
            }

            _ => {}
        }

        Ok(None)
    }

    fn handle_token_input(
        &self,
        key: crossterm::event::KeyEvent,
        state: Arc<RwLock<AppState>>,
    ) -> Result<()> {
        use crossterm::event::KeyModifiers;

        match key.code {
            KeyCode::Enter => {
                let mut s = state.write().unwrap();
                let token = s.token_input.trim().to_string();

                if !token.is_empty() {
                    s.auth.set_token(token);
                    log_debug("Token saved");
                } else {
                    log_debug("Empty token, not saving");
                }
                s.input_mode = InputMode::Normal;
                s.token_input.clear();
            }
            KeyCode::Esc => {
                let mut s = state.write().unwrap();
                s.input_mode = InputMode::Normal;
                s.token_input.clear();
                log_debug("Token input cancelled");
            }
            KeyCode::Backspace => {
                let mut s = state.write().unwrap();
                s.token_input.pop();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+U: Clear entire token
                let mut s = state.write().unwrap();
                s.token_input.clear();
                log_debug("Cleared token input");
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+W: Delete word backwards (less useful for tokens, but consistent)
                let mut s = state.write().unwrap();
                if let Some(pos) = s.token_input.rfind(|c: char| !c.is_alphanumeric()) {
                    s.token_input.truncate(pos);
                } else {
                    s.token_input.clear();
                }
            }
            KeyCode::Char(c) => {
                // Collect this character and any pending characters (for paste support)
                let mut chars = vec![c];

                // Drain any immediately available character events
                loop {
                    match event::poll(std::time::Duration::from_millis(0)) {
                        Ok(true) => {
                            if let Ok(Event::Key(next_key)) = event::read() {
                                match next_key.code {
                                    KeyCode::Char(next_c)
                                        if !next_key.modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
                                        chars.push(next_c);
                                    }
                                    _ => {
                                        // Non-character or control key, stop batching and handle it next iteration
                                        break;
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }

                // Log before consuming chars
                let char_count = chars.len();

                // Add all batched characters at once
                let mut s = state.write().unwrap();
                for ch in chars {
                    // chars is moved here, which is fine
                    s.token_input.push(ch);
                }

                if char_count > 1 {
                    log_debug(&format!(
                        "Batched {} characters (paste detected)",
                        char_count
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_clear_confirmation(
        &self,
        key: crossterm::event::KeyEvent,
        state: Arc<RwLock<AppState>>,
    ) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let mut s = state.write().unwrap();
                s.auth.clear_token();
                s.input_mode = InputMode::Normal;
                log_debug("Token cleared");
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                let mut s = state.write().unwrap();
                s.input_mode = InputMode::Normal;
                log_debug("Token clear cancelled");
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_auth_dialog(&self, state: Arc<RwLock<AppState>>) {
        let mut s = state.write().unwrap();
        s.input_mode = InputMode::EnteringToken;

        // Pre-fill with current token if exists
        s.token_input = s.auth.token.clone().unwrap_or_default();

        log_debug("Entering token input mode");
    }

    fn handle_retry(&self, state: Arc<RwLock<AppState>>) -> bool {
        let state_read = state.read().unwrap();
        if matches!(
            state_read.loading_state,
            crate::types::LoadingState::Error(_)
        ) {
            drop(state_read);

            // Increment retry count
            if let Ok(mut s) = state.write() {
                s.retry_count += 1;
            }

            return true; // Signal that we should fetch
        }
        false // Don't fetch if not in error state
    }

    fn handle_toggle_view(&mut self, state: Arc<RwLock<AppState>>, list_state: &mut ListState) {
        let mut state = state.write().unwrap();

        // Toggle view mode
        state.view_mode = match state.view_mode {
            ViewMode::Flat => ViewMode::Grouped,
            ViewMode::Grouped => ViewMode::Flat,
        };

        // Reset selection to top
        self.selected_index = 0;
        list_state.select(Some(0));

        log_debug(&format!("Switched to {:?} mode", state.view_mode));
    }

    fn handle_up(&mut self, state: Arc<RwLock<AppState>>, list_state: &mut ListState) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            list_state.select(Some(self.selected_index));

            // Reset parameter selection when changing endpoints
            let mut s = state.write().unwrap();
            s.selected_param_index = 0;
            drop(s);

            self.ensure_request_config_for_selected(state);
        }
    }

    fn handle_down(&mut self, state: Arc<RwLock<AppState>>, list_state: &mut ListState) {
        let state_guard = state.read().unwrap();

        let max_index = match state_guard.view_mode {
            ViewMode::Flat => state_guard.endpoints.len().saturating_sub(1),
            ViewMode::Grouped => state_guard.render_items.len().saturating_sub(1),
        };
        drop(state_guard);

        if self.selected_index < max_index {
            self.selected_index += 1;
            list_state.select(Some(self.selected_index));

            // Reset parameter selection when changing endpoints
            let mut s = state.write().unwrap();
            s.selected_param_index = 0;
            drop(s);

            self.ensure_request_config_for_selected(state);
        }
    }

    fn handle_enter(
        &mut self,
        state: Arc<RwLock<AppState>>,
        list_state: &mut ListState,
        base_url: Option<String>,
    ) {
        let state_read = state.read().unwrap();

        // Check what view mode we're in
        if state_read.view_mode == ViewMode::Flat {
            // In flat mode: Execute request
            if let Some(endpoint) = state_read.endpoints.get(self.selected_index) {
                let endpoint = endpoint.clone();

                // Check if we have base_url configured
                if let Some(base_url) = base_url {
                    // Check if this endpoint is already executing
                    if let Some(ref executing) = state_read.executing_endpoint {
                        if executing == &endpoint.path {
                            log_debug("Request already in progress for this endpoint");
                            return;
                        }
                    }

                    // Validate that all required path params are filled
                    let config = state_read.request_configs.get(&endpoint.path);
                    if let Err(err_msg) = can_execute_endpoint(&endpoint, config) {
                        log_debug(&format!("Cannot execute: {}", err_msg));
                        drop(state_read);

                        // Store error in response so user can see it
                        let mut s = state.write().unwrap();
                        s.current_response = Some(ApiResponse::error(err_msg));
                        return;
                    }

                    drop(state_read); // Release lock before spawning task

                    log_debug(&format!("Executing: {} {}", endpoint.method, endpoint.path));
                    execute_request_background(state.clone(), endpoint, base_url);
                } else {
                    log_debug("Cannot execute: Base URL not configured");
                }
            }
        } else {
            // In grouped mode: Check if we're on a group header or endpoint
            if let Some(item) = state_read.render_items.get(self.selected_index) {
                match item {
                    RenderItem::GroupHeader { name, .. } => {
                        let group_name = name.clone();

                        drop(state_read);
                        let mut state_write = state.write().unwrap();

                        if state_write.expanded_groups.contains(&group_name) {
                            state_write.expanded_groups.remove(&group_name);
                            log_debug(&format!("Collapsed group: {}", group_name));
                        } else {
                            state_write.expanded_groups.insert(group_name.clone());
                            log_debug(&format!("Expanded group: {}", group_name));
                        }

                        let visible_count = count_visible_items(&state_write);
                        if self.selected_index >= visible_count {
                            self.selected_index = visible_count.saturating_sub(1);
                            list_state.select(Some(self.selected_index));
                        }
                    }
                    RenderItem::Endpoint { endpoint } => {
                        let endpoint = endpoint.clone();

                        // Check if we have base_url configured
                        if let Some(base_url) = base_url {
                            // Check if this endpoint is already executing
                            if let Some(ref executing) = state_read.executing_endpoint {
                                if executing == &endpoint.path {
                                    log_debug("Request already in progress for this endpoint");
                                    return;
                                }
                            }

                            // Validate that all required path params are filled
                            let config = state_read.request_configs.get(&endpoint.path);
                            if let Err(err_msg) = can_execute_endpoint(&endpoint, config) {
                                log_debug(&format!("Cannot execute: {}", err_msg));
                                drop(state_read);

                                // Store error in response so user can see it
                                let mut s = state.write().unwrap();
                                s.current_response = Some(ApiResponse::error(err_msg));
                                return;
                            }

                            drop(state_read);

                            log_debug(&format!("Executing: {} {}", endpoint.method, endpoint.path));
                            crate::request::execute_request_background(
                                state.clone(),
                                endpoint,
                                base_url,
                            );
                        } else {
                            log_debug("Cannot execute: Base URL not configured");
                        }
                    }
                }
            }
        }
    }

    fn handle_request_param_up(&mut self, state: Arc<RwLock<AppState>>) {
        let mut s = state.write().unwrap();

        // Only navigate if in Viewing mode
        if matches!(s.request_edit_mode, RequestEditMode::Viewing) {
            if s.selected_param_index > 0 {
                s.selected_param_index -= 1;
            }
        }
    }

    fn handle_request_param_down(&mut self, state: Arc<RwLock<AppState>>) {
        let state_read = state.read().unwrap();

        // Only navigate if in Viewing mode
        if !matches!(state_read.request_edit_mode, RequestEditMode::Viewing) {
            return;
        }

        // Get currently selected endpoint to count params
        let selected_endpoint = state_read.get_selected_endpoint(self.selected_index);

        if let Some(endpoint) = selected_endpoint {
            let path_param_count = endpoint.path_params().len();
            let query_param_count = endpoint.query_params().len();
            let total_param_count = path_param_count + query_param_count;

            drop(state_read);
            let mut s = state.write().unwrap();

            if s.selected_param_index < total_param_count.saturating_sub(1) {
                s.selected_param_index += 1;
            }
        }
    }

    fn handle_request_param_edit(&mut self, state: Arc<RwLock<AppState>>) {
        // First, gather all the data we need while holding read lock
        let edit_data = {
            let state_read = state.read().unwrap();

            // Only enter edit mode if currently in Viewing mode
            if !matches!(state_read.request_edit_mode, RequestEditMode::Viewing) {
                return;
            }

            // Get currently selected endpoint
            let selected_endpoint = state_read.get_selected_endpoint(self.selected_index);

            if let Some(endpoint) = selected_endpoint {
                // Get both path and query parameters
                let path_params: Vec<_> = endpoint.path_params();
                let query_params: Vec<_> = endpoint.query_params();

                let path_param_count = path_params.len();
                let selected_idx = state_read.selected_param_index;

                // Determine if we're editing a path or query param
                let param = if selected_idx < path_param_count {
                    // We're in the path params section
                    path_params.get(selected_idx)
                } else {
                    // We're in the query params section
                    let query_idx = selected_idx - path_param_count;
                    query_params.get(query_idx)
                };

                if let Some(param) = param {
                    let param_name = param.name.clone();
                    let param_location = param.location.clone();
                    let endpoint_path = endpoint.path.clone();

                    // Get current value from the appropriate HashMap
                    let current_value = state_read
                        .request_configs
                        .get(&endpoint_path)
                        .and_then(|config| {
                            if param_location == "path" {
                                config.path_params.get(&param_name)
                            } else {
                                config.query_params.get(&param_name)
                            }
                        })
                        .cloned()
                        .unwrap_or_default();

                    Some((param_name, endpoint_path, current_value))
                } else {
                    None
                }
            } else {
                None
            }
        }; // state_read is dropped here

        // Now we can safely acquire write lock with the data we collected
        if let Some((param_name, endpoint_path, current_value)) = edit_data {
            let mut s = state.write().unwrap();

            // Ensure config exists
            s.request_configs
                .entry(endpoint_path)
                .or_insert_with(RequestConfig::default);

            // Enter edit mode
            s.request_edit_mode = RequestEditMode::Editing(param_name.clone());
            s.param_edit_buffer = current_value;

            log_debug(&format!("Editing parameter: {}", param_name));
        }
    }

    fn handle_request_param_confirm(&mut self, state: Arc<RwLock<AppState>>) {
        let state_read = state.read().unwrap();

        // Get the param name we're editing
        if let RequestEditMode::Editing(param_name) = &state_read.request_edit_mode {
            let param_name = param_name.clone();
            let new_value = state_read.param_edit_buffer.clone();

            // Get currently selected endpoint
            let selected_endpoint = state_read.get_selected_endpoint(self.selected_index);

            if let Some(endpoint) = selected_endpoint {
                let endpoint_path = endpoint.path.clone();

                // Determine which parameter we're editing by finding it in the endpoint
                let param_location = endpoint
                    .parameters
                    .iter()
                    .find(|p| p.name == param_name)
                    .map(|p| p.location.clone());

                drop(state_read);
                let mut s = state.write().unwrap();

                // Update the request config in the correct HashMap
                let config = s
                    .request_configs
                    .entry(endpoint_path)
                    .or_insert_with(RequestConfig::default);

                if let Some(location) = param_location {
                    if location == "path" {
                        config
                            .path_params
                            .insert(param_name.clone(), new_value.clone());
                    } else if location == "query" {
                        config
                            .query_params
                            .insert(param_name.clone(), new_value.clone());
                    }
                }

                // Exit edit mode
                s.request_edit_mode = RequestEditMode::Viewing;
                s.param_edit_buffer.clear();

                log_debug(&format!(
                    "Confirmed parameter {}: {}",
                    param_name, new_value
                ));
            }
        }
    }

    fn handle_request_param_cancel(&mut self, state: Arc<RwLock<AppState>>) {
        let mut s = state.write().unwrap();

        // Just exit edit mode without saving
        s.request_edit_mode = RequestEditMode::Viewing;
        s.param_edit_buffer.clear();

        log_debug("Cancelled parameter edit");
    }

    // Add this to EventHandler impl in events.rs
    fn ensure_request_config_for_selected(&self, state: Arc<RwLock<AppState>>) {
        let state_read = state.read().unwrap();

        // Get currently selected endpoint
        let selected_endpoint = state_read.get_selected_endpoint(self.selected_index);

        if let Some(endpoint) = selected_endpoint {
            let endpoint = endpoint.clone();
            drop(state_read);

            let mut s = state.write().unwrap();
            s.get_or_create_request_config(&endpoint);
        }
    }

    fn handle_search_activate(&self, state: Arc<RwLock<AppState>>) {
        let mut s = state.write().unwrap();
        s.input_mode = InputMode::Searching;
        log_debug("Entering search mode");
    }

    fn handle_search_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        state: Arc<RwLock<AppState>>,
        list_state: &mut ListState,
    ) -> Result<()> {
        use crossterm::event::KeyModifiers;

        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                // Exit search mode but keep the filter active
                let mut s = state.write().unwrap();
                s.input_mode = InputMode::Normal;
                log_debug("Exiting search mode");
            }
            KeyCode::Backspace => {
                let mut s = state.write().unwrap();
                s.search_query.pop();
                s.update_filtered_endpoints();

                log_debug(&format!("Search query: '{}'", s.search_query));

                // Reset selection to top
                drop(s);
                self.selected_index = 0;
                list_state.select(Some(0));
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Clear search
                let mut s = state.write().unwrap();
                s.search_query.clear();
                s.update_filtered_endpoints();
                log_debug("Cleared search query");

                drop(s);
                self.selected_index = 0;
                list_state.select(Some(0));
            }
            KeyCode::Char(c) => {
                let mut s = state.write().unwrap();
                s.search_query.push(c);
                s.update_filtered_endpoints();

                log_debug(&format!("Search query: '{}'", s.search_query));

                // Reset selection to top when search changes
                drop(s);
                self.selected_index = 0;
                list_state.select(Some(0));
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_search_clear(&mut self, state: Arc<RwLock<AppState>>, list_state: &mut ListState) {
        let mut s = state.write().unwrap();
        if !s.search_query.is_empty() {
            s.search_query.clear();
            s.update_filtered_endpoints();
            log_debug("Cleared search filter");

            drop(s);
            self.selected_index = 0;
            list_state.select(Some(0));
        }
    }
}

/// Check if endpoint can be executed (all required path params are filled)
fn can_execute_endpoint(
    endpoint: &ApiEndpoint,
    config: Option<&RequestConfig>,
) -> Result<(), String> {
    // Check if we have a config at all
    let config = match config {
        Some(c) => c,
        None => return Err("No request configuration found".to_string()),
    };

    // Check if all path params are filled
    if !endpoint.has_all_required_path_params(config) {
        let missing = endpoint.missing_path_params(config);
        return Err(format!(
            "Missing required path parameter(s): {}",
            missing.join(", ")
        ));
    }

    Ok(())
}

fn log_debug(msg: &str) {
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/lazy-swagger-tui.log")
        .and_then(|mut f| writeln!(f, "{}", msg));
}
