use crate::config;
use crate::state::{AppState, count_visible_items};
use crate::types::{InputMode, RenderItem, ViewMode};

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
    ) -> Result<(bool, Option<String>)> {
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
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            // ← Add this
                            self.handle_url_dialog(state.clone());
                        }
                        KeyCode::F(5) => {
                            should_fetch = true;
                        }
                        KeyCode::Up => {
                            self.handle_up(list_state);
                        }
                        KeyCode::Down => {
                            self.handle_down(state.clone(), list_state);
                        }
                        KeyCode::Enter => {
                            self.handle_enter(state.clone(), list_state);
                        }
                        _ => {}
                    },
                }
            }
        }
        Ok((should_fetch, url_submitted))
    }

    fn handle_url_dialog(&self, state: Arc<RwLock<AppState>>) {
        let mut s = state.write().unwrap();
        s.input_mode = InputMode::EnteringUrl;
        // Pre-fill with current URL if it exists (we'll need to pass this from App)
        // For now, start empty
        s.url_input.clear();
        log_debug("Entering URL input mode");
    }

    fn handle_url_input(
        &self,
        key: crossterm::event::KeyEvent,
        state: Arc<RwLock<AppState>>,
    ) -> Result<Option<String>> {
        match key.code {
            KeyCode::Enter => {
                let mut s = state.write().unwrap();
                let url = s.url_input.trim().to_string();

                if !url.is_empty() {
                    // Validate URL
                    match config::validate_url(&url) {
                        Ok(_) => {
                            s.input_mode = InputMode::Normal;
                            s.url_input.clear();
                            log_debug(&format!("URL submitted: {}", url));
                            return Ok(Some(url));
                        }
                        Err(e) => {
                            log_debug(&format!("Invalid URL: {}", e));
                            // Keep the modal open, user can fix it
                            // TODO: Show error message in modal
                        }
                    }
                } else {
                    log_debug("Empty URL, not submitting");
                }
            }
            KeyCode::Esc => {
                let mut s = state.write().unwrap();
                s.input_mode = InputMode::Normal;
                s.url_input.clear();
                log_debug("URL input cancelled");
            }
            KeyCode::Backspace => {
                let mut s = state.write().unwrap();
                s.url_input.pop();
            }
            KeyCode::Char(c) => {
                let mut s = state.write().unwrap();
                s.url_input.push(c);
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
            KeyCode::Char(c) => {
                let mut s = state.write().unwrap();
                s.token_input.push(c);
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

    fn handle_enter(&mut self, state: Arc<RwLock<AppState>>, list_state: &mut ListState) {
        let state_read = state.read().unwrap();

        // Check what view mode we're in
        if state_read.view_mode == ViewMode::Flat {
            // In flat mode: Execute request
            // TODO: Execute request for selected endpoint
            log_debug("Execute request in flat mode");
        } else {
            // In grouped mode: Check if we're on a group header or endpoint
            if let Some(item) = state_read.render_items.get(self.selected_index) {
                match item {
                    RenderItem::GroupHeader { name, .. } => {
                        let group_name = name.clone();

                        drop(state_read); // Release read lock
                        let mut state_write = state.write().unwrap();

                        if state_write.expanded_groups.contains(&group_name) {
                            state_write.expanded_groups.remove(&group_name);
                            log_debug(&format!("Collapsed group: {}", group_name));
                        } else {
                            state_write.expanded_groups.insert(group_name.clone());
                            log_debug(&format!("Expanded group: {}", group_name));
                        }

                        // Validate selection is still in bounds
                        let visible_count = count_visible_items(&state_write);
                        if self.selected_index >= visible_count {
                            self.selected_index = visible_count.saturating_sub(1);
                            list_state.select(Some(self.selected_index));
                        }
                    }
                    RenderItem::Endpoint { endpoint } => {
                        // Execute request for this endpoint
                        log_debug(&format!("Execute: {} {}", endpoint.method, endpoint.path));
                        // TODO: Actually execute request
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
