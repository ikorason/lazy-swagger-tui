//! Navigation handlers
//!
//! This module handles navigation through the UI:
//! - List navigation (up/down in endpoints list)
//! - Parameter navigation (j/k in request params)
//! - Response line navigation (j/k in response viewer)
//! - View mode toggling (flat vs grouped)

use super::helpers::{apply, log_debug};
use crate::actions::AppAction;
use crate::state::AppState;
use crate::types::{RequestEditMode, ViewMode};
use crate::ui::draw::try_format_json;
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

        // Reset parameter selection and response scroll when changing endpoints
        let mut s = state.write().unwrap();
        s.ui.selected_param_index = 0;
        s.ui.response_scroll = 0;
        s.ui.response_selected_line = 0;
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

        // Reset parameter selection and response scroll when changing endpoints
        let mut s = state.write().unwrap();
        s.ui.selected_param_index = 0;
        s.ui.response_scroll = 0;
        s.ui.response_selected_line = 0;
        drop(s);

        ensure_request_config_for_selected(*selected_index, state);
    }
}

/// Navigate up in request parameters
pub fn handle_request_param_up(state: Arc<RwLock<AppState>>) {
    let mut s = state.write().unwrap();

    // Only navigate if in Viewing mode
    if matches!(s.request.edit_mode, RequestEditMode::Viewing)
        && s.ui.selected_param_index > 0 {
            s.ui.selected_param_index -= 1;
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
    log_debug(&format!("Switched to {view_mode:?} mode"));
}

/// Navigate up in response lines
pub fn handle_response_line_up(state: Arc<RwLock<AppState>>) {
    let mut s = state.write().unwrap();

    if s.ui.response_selected_line > 0 {
        s.ui.response_selected_line -= 1;

        // Adjust scroll if selected line goes above viewport
        if s.ui.response_selected_line < s.ui.response_scroll {
            s.ui.response_scroll = s.ui.response_selected_line;
        }
    }
}

/// Navigate down in response lines
pub fn handle_response_line_down(state: Arc<RwLock<AppState>>) {
    let state_read = state.read().unwrap();

    // Count total lines in response
    let total_lines = if let Some(ref response) = state_read.request.current_response {
        if !response.is_error {
            // Count lines in formatted JSON (status + empty + body lines)
            let formatted_body = try_format_json(&response.body);
            2 + formatted_body.lines().count()
        } else {
            0
        }
    } else {
        0
    };

    drop(state_read);
    let mut s = state.write().unwrap();

    if total_lines > 0 && s.ui.response_selected_line < total_lines - 1 {
        s.ui.response_selected_line += 1;

        // Auto-scroll down to keep selection visible (assume 20 line viewport for now)
        let viewport_height = 20;
        let scroll_bottom = s.ui.response_scroll + viewport_height;
        if s.ui.response_selected_line >= scroll_bottom {
            s.ui.response_scroll =
                s.ui.response_selected_line
                    .saturating_sub(viewport_height - 1);
        }
    }
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
