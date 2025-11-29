//! Event handling system for lazy-swagger-tui
//!
//! This module processes user input and translates it into state-changing actions.
//! It handles multiple input modes:
//! - Normal: Standard navigation and commands
//! - EnteringUrl: Modal for configuring API URLs
//! - EnteringToken: Modal for bearer token authentication
//! - Searching: Filtering endpoints by query
//! - Parameter editing: Inline editing of request parameters
//!
//! # Architecture
//!
//! The EventHandler uses an action pattern where input events generate AppActions
//! that are applied to AppState via the apply_action function in actions.rs.
//!
//! # Lock Management
//!
//! This module frequently acquires locks on Arc<RwLock<AppState>>. Care must be
//! taken to minimize lock duration and avoid deadlocks. See handle_events for
//! the main event loop.

mod execution;
mod helpers;
mod modals;
mod navigation;
mod parameters;
mod search;
mod yank;

// Re-export public items
pub use helpers::{apply, apply_or_char, is_editing, log_debug};

use crate::actions::AppAction;
use crate::state::AppState;
use crate::types::{DetailTab, InputMode, PanelFocus, RequestEditMode, UrlSubmission};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::widgets::ListState;
use std::sync::{Arc, RwLock};

/// Event handler for managing user input and state updates
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

    /// Main event handling loop - dispatches to appropriate handlers based on input mode
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
                let input_mode = state.read().unwrap().input.mode.clone();

                match input_mode {
                    InputMode::EnteringUrl => {
                        url_submitted = modals::handle_url_input(key, state.clone())?;
                    }

                    InputMode::EnteringToken => {
                        modals::handle_token_input(key, state.clone())?;
                    }

                    InputMode::ConfirmClearToken => {
                        modals::handle_clear_confirmation(key, state.clone())?;
                    }

                    InputMode::Searching => {
                        search::handle_search_input(
                            &mut self.selected_index,
                            key,
                            state.clone(),
                            list_state,
                        )?;
                    }

                    InputMode::EnteringBody => {
                        modals::handle_body_input(key, state.clone(), self.selected_index)?;
                    }

                    InputMode::Normal => match key.code {
                        // QUIT
                        KeyCode::Char('q') => {
                            // Don't quit if we're editing a parameter
                            if !is_editing(&state) {
                                // Not editing - quit the app
                                self.should_quit = true;
                            } else {
                                // We're editing - treat 'q' as character input
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('q');
                            }
                        }
                        // nav down
                        KeyCode::Char('j') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('j');
                            } else {
                                let state_read = state.read().unwrap();
                                let panel = state_read.ui.panel_focus.clone();
                                let active_tab = state_read.ui.active_detail_tab.clone();
                                drop(state_read);

                                // Handle navigation
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        // Navigate down in endpoints list
                                        navigation::handle_down(
                                            &mut self.selected_index,
                                            state.clone(),
                                            list_state,
                                        );
                                    }
                                    PanelFocus::Details => {
                                        // If on Request tab and in Viewing mode, navigate params
                                        if active_tab == DetailTab::Request {
                                            navigation::handle_request_param_down(
                                                self.selected_index,
                                                state.clone(),
                                            );
                                        } else if active_tab == DetailTab::Response {
                                            navigation::handle_response_line_down(state.clone());
                                        }
                                        // For other tabs, j/k do nothing
                                    }
                                }
                            }
                        }
                        // nav up
                        KeyCode::Char('k') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('k');
                            } else {
                                let state_read = state.read().unwrap();
                                let panel = state_read.ui.panel_focus.clone();
                                let active_tab = state_read.ui.active_detail_tab.clone();
                                drop(state_read);

                                // Handle navigation
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        // Navigate up in endpoints list
                                        navigation::handle_up(
                                            &mut self.selected_index,
                                            state.clone(),
                                            list_state,
                                        );
                                    }
                                    PanelFocus::Details => {
                                        // If on Request tab and in Viewing mode, navigate params
                                        if active_tab == DetailTab::Request {
                                            navigation::handle_request_param_up(state.clone());
                                        } else if active_tab == DetailTab::Response {
                                            navigation::handle_response_line_up(state.clone());
                                        }
                                        // For other tabs, j/k do nothing
                                    }
                                }
                            }
                        }
                        // handle auth dialog
                        KeyCode::Char('a') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('a');
                            } else {
                                modals::handle_auth_dialog(state.clone());
                            }
                        }
                        // handle body editor
                        KeyCode::Char('b') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('b');
                            } else {
                                let state_read = state.read().unwrap();
                                let panel = state_read.ui.panel_focus.clone();
                                let active_tab = state_read.ui.active_detail_tab.clone();
                                let supports_body = state_read
                                    .get_selected_endpoint(self.selected_index)
                                    .map(|ep| ep.supports_body())
                                    .unwrap_or(false);
                                drop(state_read);

                                if panel == PanelFocus::Details
                                    && active_tab == DetailTab::Request
                                    && supports_body
                                {
                                    modals::handle_body_dialog(state.clone(), self.selected_index);
                                }
                            }
                        }
                        // edit param
                        KeyCode::Char('e') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('e');
                            } else {
                                let state_read = state.read().unwrap();
                                let panel = state_read.ui.panel_focus.clone();
                                let active_tab = state_read.ui.active_detail_tab.clone();
                                drop(state_read);

                                if panel == PanelFocus::Details && active_tab == DetailTab::Request {
                                    parameters::handle_request_param_edit(
                                        self.selected_index,
                                        state.clone(),
                                    );
                                }
                            }
                        }
                        // toggle view - list <-> grouped
                        KeyCode::Char('g') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('g');
                            } else {
                                navigation::handle_toggle_view(
                                    &mut self.selected_index,
                                    state.clone(),
                                    list_state,
                                );
                            }
                        }
                        // config url
                        KeyCode::Char(',') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push(',');
                            } else {
                                modals::handle_url_dialog(
                                    state.clone(),
                                    swagger_url.clone(),
                                    base_url.clone(),
                                );
                            }
                        }
                        // search endpoints
                        KeyCode::Char('/') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('/');
                            } else {
                                search::handle_search_activate(state.clone());
                            }
                        }
                        // toggle body section
                        KeyCode::Char('x') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('x');
                            } else {
                                let state_read = state.read().unwrap();
                                let panel = state_read.ui.panel_focus.clone();
                                let active_tab = state_read.ui.active_detail_tab.clone();
                                let supports_body = state_read
                                    .get_selected_endpoint(self.selected_index)
                                    .map(|ep| ep.supports_body())
                                    .unwrap_or(false);
                                drop(state_read);

                                if panel == PanelFocus::Details
                                    && active_tab == DetailTab::Request
                                    && supports_body
                                {
                                    apply(state.clone(), AppAction::ToggleBodySection);
                                }
                            }
                        }
                        // yank (copy) current line
                        KeyCode::Char('y') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push('y');
                            } else {
                                let state_read = state.read().unwrap();
                                let panel = state_read.ui.panel_focus.clone();
                                let active_tab = state_read.ui.active_detail_tab.clone();
                                drop(state_read);

                                if panel == PanelFocus::Details && active_tab == DetailTab::Response {
                                    yank::handle_yank_response_line(state.clone());
                                }
                            }
                        }
                        // switch to endpoints panel
                        KeyCode::Char('1') => {
                            apply_or_char(
                                state.clone(),
                                '1',
                                AppAction::NavigateToPanel(PanelFocus::EndpointsList),
                            );
                        }
                        // switch to details panel
                        KeyCode::Char('2') => {
                            apply_or_char(
                                state.clone(),
                                '2',
                                AppAction::NavigateToPanel(PanelFocus::Details),
                            );
                        }

                        // ctrl + modifiers
                        // retry
                        KeyCode::Char('r')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            should_fetch = execution::handle_retry(state.clone());
                        }

                        // Ctrl+l: Clear search filter
                        KeyCode::Char('l')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            search::handle_search_clear(
                                &mut self.selected_index,
                                state.clone(),
                                list_state,
                            );
                        }

                        // Special keys --
                        // tab navigation
                        KeyCode::Tab => {
                            apply(state.clone(), AppAction::NavigateTabForward);
                        }
                        // Shift+Tab (BackTab) - move left
                        KeyCode::BackTab => {
                            apply(state.clone(), AppAction::NavigateTabBackward);
                        }
                        // space  - execute & expand
                        KeyCode::Char(' ') => {
                            if is_editing(&state) {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push(' ');
                            } else {
                                execution::handle_enter(
                                    &mut self.selected_index,
                                    state.clone(),
                                    list_state,
                                    base_url.clone(),
                                );
                            }
                        }
                        // enter - param confirm
                        KeyCode::Enter => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.ui.panel_focus.clone();
                            let active_tab = state_read.ui.active_detail_tab.clone();
                            let edit_mode = state_read.request.edit_mode.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            // ONLY handle if on Request tab and in Editing mode
                            if panel == PanelFocus::Details
                                && active_tab == DetailTab::Request
                                && matches!(edit_mode, RequestEditMode::Editing(_))
                            {
                                parameters::handle_request_param_confirm(
                                    self.selected_index,
                                    state.clone(),
                                );
                            }
                        }
                        // backspace - param edit
                        KeyCode::Backspace => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.ui.panel_focus.clone();
                            let active_tab = state_read.ui.active_detail_tab.clone();
                            let edit_mode = state_read.request.edit_mode.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            // ONLY handle if on Request tab and in Editing mode
                            if panel == PanelFocus::Details
                                && active_tab == DetailTab::Request
                                && matches!(edit_mode, RequestEditMode::Editing(_))
                            {
                                apply(state.clone(), AppAction::BackspaceParamBuffer);
                            }
                        }
                        // esc - cancel param edit
                        KeyCode::Esc => {
                            let state_read = state.read().unwrap();
                            let panel = state_read.ui.panel_focus.clone();
                            let active_tab = state_read.ui.active_detail_tab.clone();
                            let edit_mode = state_read.request.edit_mode.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            // ONLY handle if on Request tab and in Editing mode
                            if panel == PanelFocus::Details
                                && active_tab == DetailTab::Request
                                && matches!(edit_mode, RequestEditMode::Editing(_))
                            {
                                apply(state.clone(), AppAction::CancelParameterEdit);
                            }
                        }

                        // keep arrow keys for accessibility (optional)
                        KeyCode::Up => {
                            if !is_editing(&state) {
                                let state_read = state.read().unwrap();
                                let panel = state_read.ui.panel_focus.clone();
                                let active_tab = state_read.ui.active_detail_tab.clone();
                                drop(state_read);

                                use crate::types::PanelFocus;
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        navigation::handle_up(
                                            &mut self.selected_index,
                                            state.clone(),
                                            list_state,
                                        );
                                    }
                                    PanelFocus::Details => {
                                        if active_tab == DetailTab::Request {
                                            navigation::handle_request_param_up(state.clone());
                                        }
                                    }
                                }
                            }
                        }

                        KeyCode::Down => {
                            if !is_editing(&state) {
                                let state_read = state.read().unwrap();
                                let panel = state_read.ui.panel_focus.clone();
                                let active_tab = state_read.ui.active_detail_tab.clone();
                                drop(state_read);

                                use crate::types::PanelFocus;
                                match panel {
                                    PanelFocus::EndpointsList => {
                                        navigation::handle_down(
                                            &mut self.selected_index,
                                            state.clone(),
                                            list_state,
                                        );
                                    }
                                    PanelFocus::Details => {
                                        if active_tab == DetailTab::Request {
                                            navigation::handle_request_param_down(
                                                self.selected_index,
                                                state.clone(),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        KeyCode::Char(c)
                            if !key.modifiers.contains(KeyModifiers::CONTROL) && c != ' ' =>
                        {
                            let state_read = state.read().unwrap();
                            let panel = state_read.ui.panel_focus.clone();
                            let active_tab = state_read.ui.active_detail_tab.clone();
                            let edit_mode = state_read.request.edit_mode.clone();
                            drop(state_read);

                            use crate::types::PanelFocus;

                            if panel == PanelFocus::Details
                                && active_tab == DetailTab::Request
                                && matches!(edit_mode, RequestEditMode::Editing(_))
                            {
                                let mut s = state.write().unwrap();
                                s.request.param_edit_buffer.push(c);
                                log_debug(&format!(
                                    "Added char, buffer now: {}",
                                    s.request.param_edit_buffer
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
}
