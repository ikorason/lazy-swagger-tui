//! Navigation handlers
//!
//! This module handles navigation through the UI:
//! - List navigation (up/down in endpoints list)
//! - Parameter navigation (j/k in request params)
//! - View mode toggling (flat vs grouped)

use super::helpers::{apply, log_debug};
use crate::actions::AppAction;
use crate::state::AppState;
use crate::types::{RequestEditMode, ViewMode};
use ratatui::widgets::ListState;
use std::sync::{Arc, RwLock};

/// Navigate up in endpoints list
pub fn handle_up(
    selected_index: &mut usize,
    state: Arc<RwLock<AppState>>,
    list_state: &mut ListState,
) {
    if *selected_index > 0 {
        *selected_index -= 1;
        list_state.select(Some(*selected_index));

        // Reset parameter selection when changing endpoints
        let mut s = state.write().unwrap();
        s.ui.selected_param_index = 0;
        drop(s);

        ensure_request_config_for_selected(*selected_index, state);
    }
}

/// Navigate down in endpoints list
pub fn handle_down(
    selected_index: &mut usize,
    state: Arc<RwLock<AppState>>,
    list_state: &mut ListState,
) {
    let state_guard = state.read().unwrap();

    let max_index = match state_guard.ui.view_mode {
        ViewMode::Flat => state_guard.data.endpoints.len().saturating_sub(1),
        ViewMode::Grouped => state_guard.get_render_items().len().saturating_sub(1),
    };
    drop(state_guard);

    if *selected_index < max_index {
        *selected_index += 1;
        list_state.select(Some(*selected_index));

        // Reset parameter selection when changing endpoints
        let mut s = state.write().unwrap();
        s.ui.selected_param_index = 0;
        drop(s);

        ensure_request_config_for_selected(*selected_index, state);
    }
}

/// Navigate up in request parameters
pub fn handle_request_param_up(state: Arc<RwLock<AppState>>) {
    let mut s = state.write().unwrap();

    // Only navigate if in Viewing mode
    if matches!(s.request.edit_mode, RequestEditMode::Viewing) {
        if s.ui.selected_param_index > 0 {
            s.ui.selected_param_index -= 1;
        }
    }
}

/// Navigate down in request parameters
pub fn handle_request_param_down(selected_index: usize, state: Arc<RwLock<AppState>>) {
    let state_read = state.read().unwrap();

    // Only navigate if in Viewing mode
    if !matches!(state_read.request.edit_mode, RequestEditMode::Viewing) {
        return;
    }

    // Get currently selected endpoint to count params
    let selected_endpoint = state_read.get_selected_endpoint(selected_index);

    if let Some(endpoint) = selected_endpoint {
        let path_param_count = endpoint.path_params().len();
        let query_param_count = endpoint.query_params().len();
        let total_param_count = path_param_count + query_param_count;

        drop(state_read);
        let mut s = state.write().unwrap();

        if s.ui.selected_param_index < total_param_count.saturating_sub(1) {
            s.ui.selected_param_index += 1;
        }
    }
}

/// Toggle between flat and grouped view modes
pub fn handle_toggle_view(
    selected_index: &mut usize,
    state: Arc<RwLock<AppState>>,
    list_state: &mut ListState,
) {
    apply(state.clone(), AppAction::ToggleViewMode);

    // Reset selection to top
    *selected_index = 0;
    list_state.select(Some(0));

    let view_mode = state.read().unwrap().ui.view_mode.clone();
    log_debug(&format!("Switched to {:?} mode", view_mode));
}

/// Ensure request config exists for selected endpoint
fn ensure_request_config_for_selected(selected_index: usize, state: Arc<RwLock<AppState>>) {
    let state_read = state.read().unwrap();

    // Get currently selected endpoint
    let selected_endpoint = state_read.get_selected_endpoint(selected_index);

    if let Some(endpoint) = selected_endpoint {
        let endpoint = endpoint.clone();
        drop(state_read);

        let mut s = state.write().unwrap();
        s.get_or_create_request_config(&endpoint);
    }
}
