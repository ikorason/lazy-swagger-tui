use crate::config;
use crate::state::{AppState, count_visible_items};
use crate::types::{InputMode, RenderItem, UrlInputField, UrlSubmission, ViewMode};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
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
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            self.should_quit = true;
                        }

                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            should_fetch = self.handle_retry(state.clone());
                        }

                        KeyCode::Char('g') | KeyCode::Char('G') => {
                            self.handle_toggle_view(state.clone(), list_state);
                        }

                        KeyCode::Char('a') => {
                            self.handle_auth_dialog(state.clone());
                        }

                        KeyCode::Char('A') => {
                            self.handle_clear_token_request(state.clone());
                        }

                        KeyCode::Tab => {
                            let mut s = state.write().unwrap();
                            use crate::types::{DetailTab, PanelFocus};

                            match s.panel_focus {
                                PanelFocus::EndpointsList => {
                                    // Move from Endpoints panel to Details panel (Endpoint tab)
                                    s.panel_focus = PanelFocus::Details;
                                    s.active_detail_tab = DetailTab::Endpoint;
                                }
                                PanelFocus::Details => {
                                    // Cycle through tabs in Details panel
                                    s.active_detail_tab = match s.active_detail_tab {
                                        DetailTab::Endpoint => DetailTab::Headers,
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

                        KeyCode::BackTab => {
                            // Shift+Tab (BackTab) - move left
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
                                    s.active_detail_tab = match s.active_detail_tab {
                                        DetailTab::Response => DetailTab::Headers,
                                        DetailTab::Headers => DetailTab::Endpoint,
                                        DetailTab::Endpoint => {
                                            // Wrap back to Endpoints panel
                                            s.panel_focus = PanelFocus::EndpointsList;
                                            DetailTab::Response // Reset for next time
                                        }
                                    };
                                }
                            }
                        }

                        KeyCode::Char('j') => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            match panel {
                                PanelFocus::EndpointsList => {
                                    // Navigate down in endpoints list
                                    self.handle_down(state.clone(), list_state);
                                }
                                PanelFocus::Details => {
                                    // TODO: Future - scroll content line by line
                                    // For now, j/k do nothing in Details panel
                                    // Use Ctrl+d/Ctrl+u for scrolling
                                }
                            }
                        }

                        KeyCode::Char('k') => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            match panel {
                                PanelFocus::EndpointsList => {
                                    // Navigate up in endpoints list
                                    self.handle_up(list_state);
                                }
                                PanelFocus::Details => {
                                    // TODO: Future - scroll content line by line
                                    // For now, j/k do nothing in Details panel
                                    // Use Ctrl+d/Ctrl+u for scrolling
                                }
                            }
                        }

                        KeyCode::Char('u')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            // Ctrl+u: Scroll up in focused section
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
                                    DetailTab::Endpoint => {
                                        // Endpoint tab doesn't scroll
                                    }
                                }
                            }
                        }

                        KeyCode::Char('d')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            // Ctrl+d: Scroll down in focused section
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
                                            s.response_body_scroll.saturating_add(5);
                                    }
                                    DetailTab::Headers => {
                                        s.headers_scroll = s.headers_scroll.saturating_add(5);
                                    }
                                    DetailTab::Endpoint => {
                                        // Endpoint tab doesn't scroll
                                    }
                                }
                            }
                        }

                        KeyCode::Char(',') => {
                            self.handle_url_dialog(
                                state.clone(),
                                swagger_url.clone(),
                                base_url.clone(),
                            );
                        }

                        KeyCode::Char(' ') => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            match panel {
                                PanelFocus::EndpointsList => {
                                    // Space executes request or expands group
                                    self.handle_enter(state.clone(), list_state, base_url.clone());
                                }
                                PanelFocus::Details => {
                                    // Space in Details panel: Execute current endpoint again
                                    let state_read = state.read().unwrap();
                                    let view_mode = state_read.view_mode.clone();

                                    let selected_endpoint = match view_mode {
                                        ViewMode::Flat => {
                                            state_read.endpoints.get(self.selected_index).cloned()
                                        }
                                        ViewMode::Grouped => state_read
                                            .render_items
                                            .get(self.selected_index)
                                            .and_then(|item| match item {
                                                RenderItem::Endpoint { endpoint } => {
                                                    Some(endpoint.clone())
                                                }
                                                RenderItem::GroupHeader { .. } => None,
                                            }),
                                    };

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

                                            drop(state_read);
                                            log_debug(&format!(
                                                "Re-executing: {} {}",
                                                endpoint.method, endpoint.path
                                            ));
                                            crate::request::execute_request_background(
                                                state.clone(),
                                                endpoint,
                                                base_url,
                                            );
                                        } else {
                                            log_debug("Cannot execute: Base URL not configured");
                                        }
                                    } else {
                                        log_debug("No endpoint selected (group header or empty)");
                                    }
                                }
                            }
                        }

                        // keep arrow keys for accessibility (optional)
                        KeyCode::Up => {
                            // Only works when in EndpointsList
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;
                            if panel == PanelFocus::EndpointsList {
                                self.handle_up(list_state);
                            }
                        }

                        KeyCode::Down => {
                            // Only works when in EndpointsList
                            let state_read = state.read().unwrap();
                            let panel = state_read.panel_focus.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;
                            if panel == PanelFocus::EndpointsList {
                                self.handle_down(state.clone(), list_state);
                            }
                        }

                        KeyCode::F(5) => {
                            should_fetch = true;
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

    fn handle_clear_token_request(&self, state: Arc<RwLock<AppState>>) {
        let s = state.read().unwrap();

        // Only show confirmation if token exists
        if s.auth.is_authenticated() {
            drop(s);
            let mut s = state.write().unwrap();
            s.input_mode = InputMode::ConfirmClearToken;
            log_debug("Requesting token clear confirmation");
        }
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

    fn handle_up(&mut self, list_state: &mut ListState) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            list_state.select(Some(self.selected_index));
        }
    }

    fn handle_down(&mut self, state: Arc<RwLock<AppState>>, list_state: &mut ListState) {
        let state = state.read().unwrap();

        // Use render_items length in grouped mode, endpoints length in flat mode
        let max_index = match state.view_mode {
            ViewMode::Flat => state.endpoints.len().saturating_sub(1),
            ViewMode::Grouped => state.render_items.len().saturating_sub(1),
        };

        if self.selected_index < max_index {
            self.selected_index += 1;
            list_state.select(Some(self.selected_index));
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

                    drop(state_read); // Release lock before spawning task

                    log_debug(&format!("Executing: {} {}", endpoint.method, endpoint.path));
                    crate::request::execute_request_background(state.clone(), endpoint, base_url);
                } else {
                    log_debug("Cannot execute: Base URL not configured");
                    // TODO: Show error to user
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

                            drop(state_read);

                            log_debug(&format!("Executing: {} {}", endpoint.method, endpoint.path));
                            crate::request::execute_request_background(
                                state.clone(),
                                endpoint,
                                base_url,
                            );
                        } else {
                            log_debug("Cannot execute: Base URL not configured");
                            // TODO: Show error to user
                        }
                    }
                }
            }
        }
    }
}

fn log_debug(msg: &str) {
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/dotrest.log")
        .and_then(|mut f| writeln!(f, "{}", msg));
}
