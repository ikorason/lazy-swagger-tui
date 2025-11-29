//! Search handlers
//!
//! This module handles searching/filtering endpoints:
//! - Activating search mode
//! - Handling search input
//! - Clearing search filters

use super::helpers::log_debug;
use crate::actions::{AppAction, apply_action};
use crate::state::AppState;
use crate::types::InputMode;
use color_eyre::Result;
use crossterm::event::KeyCode;
use ratatui::widgets::ListState;
use std::sync::{Arc, RwLock};

/// Activate search mode
pub fn handle_search_activate(state: Arc<RwLock<AppState>>) {
    let mut s = state.write().unwrap();
    apply_action(AppAction::EnterSearchMode, &mut s);
}

/// Handle search input
pub fn handle_search_input(
    selected_index: &mut usize,
    key: crossterm::event::KeyEvent,
    state: Arc<RwLock<AppState>>,
    list_state: &mut ListState,
) -> Result<()> {
    use crossterm::event::KeyModifiers;

    match key.code {
        KeyCode::Enter => {
            // Exit search mode and keep the filter active
            let mut s = state.write().unwrap();
            s.input.mode = InputMode::Normal;
            log_debug("Exiting search mode (keeping filter)");
        }
        KeyCode::Esc => {
            // Exit search mode and clear the filter
            let mut s = state.write().unwrap();
            s.input.mode = InputMode::Normal;
            s.search.query.clear();
            s.update_filtered_endpoints();
            log_debug("Exiting search mode (cleared filter)");

            drop(s);
            *selected_index = 0;
            list_state.select(Some(0));
        }
        KeyCode::Backspace => {
            let mut s = state.write().unwrap();
            s.search.query.pop();
            s.update_filtered_endpoints();

            log_debug(&format!("Search query: '{}'", s.search.query));

            // Reset selection to top
            drop(s);
            *selected_index = 0;
            list_state.select(Some(0));
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+L: Clear search (consistent with other inputs)
            let mut s = state.write().unwrap();
            s.search.query.clear();
            s.update_filtered_endpoints();
            log_debug("Cleared search query");

            drop(s);
            *selected_index = 0;
            list_state.select(Some(0));
        }
        KeyCode::Char(c) => {
            let mut s = state.write().unwrap();
            s.search.query.push(c);
            s.update_filtered_endpoints();

            log_debug(&format!("Search query: '{}'", s.search.query));

            // Reset selection to top when search changes
            drop(s);
            *selected_index = 0;
            list_state.select(Some(0));
        }
        _ => {}
    }
    Ok(())
}

/// Clear search filter
pub fn handle_search_clear(
    selected_index: &mut usize,
    state: Arc<RwLock<AppState>>,
    list_state: &mut ListState,
) {
    let mut s = state.write().unwrap();
    if !s.search.query.is_empty() {
        s.search.query.clear();
        s.update_filtered_endpoints();
        log_debug("Cleared search filter");

        drop(s);
        *selected_index = 0;
        list_state.select(Some(0));
    }
}
