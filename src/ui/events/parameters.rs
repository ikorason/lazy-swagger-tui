//! Parameter editing handlers
//!
//! This module handles editing of request parameters:
//! - Entering edit mode for a selected parameter
//! - Confirming parameter edits
//! - Ensuring request configs exist

use super::helpers::{apply, log_debug};
use crate::actions::AppAction;
use crate::state::AppState;
use crate::types::{RequestConfig, RequestEditMode};
use std::sync::{Arc, RwLock};

/// Enter edit mode for the currently selected parameter
pub fn handle_request_param_edit(selected_index: usize, state: Arc<RwLock<AppState>>) {
    // First, gather all the data we need while holding read lock
    let edit_data = {
        let state_read = state.read().unwrap();

        // Only enter edit mode if currently in Viewing mode
        if !matches!(state_read.request.edit_mode, RequestEditMode::Viewing) {
            return;
        }

        // Get currently selected endpoint
        let selected_endpoint = state_read.get_selected_endpoint(selected_index);

        if let Some(endpoint) = selected_endpoint {
            // Get both path and query parameters
            let path_params: Vec<_> = endpoint.path_params();
            let query_params: Vec<_> = endpoint.query_params();

            let path_param_count = path_params.len();
            let selected_idx = state_read.ui.selected_param_index;

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
                let endpoint_path = endpoint.path.clone();

                // Get current value from the appropriate HashMap
                let current_value = state_read
                    .request
                    .configs
                    .get(&endpoint_path)
                    .and_then(|config| config.get_param_value(&param_name).map(|s| s.to_string()))
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
    if let Some((param_name, endpoint_path, _current_value)) = edit_data {
        // Ensure config exists
        {
            let mut s = state.write().unwrap();
            s.request
                .configs
                .entry(endpoint_path.clone())
                .or_insert_with(RequestConfig::default);
        }

        // Enter edit mode using action
        apply(
            state.clone(),
            AppAction::StartEditingParameter {
                param_name: param_name.clone(),
                endpoint_path,
            },
        );

        log_debug(&format!("Editing parameter: {}", param_name));
    }
}

/// Confirm parameter edit and save the value
pub fn handle_request_param_confirm(selected_index: usize, state: Arc<RwLock<AppState>>) {
    let (is_editing, endpoint_path) = {
        let state_read = state.read().unwrap();

        // Check if we're editing
        let is_editing = matches!(state_read.request.edit_mode, RequestEditMode::Editing(_));

        // Get currently selected endpoint path
        let endpoint_path = state_read
            .get_selected_endpoint(selected_index)
            .map(|endpoint| endpoint.path.clone());

        (is_editing, endpoint_path)
    };

    if is_editing {
        if let Some(path) = endpoint_path {
            // Confirm the edit using action
            apply(
                state.clone(),
                AppAction::ConfirmParameterEdit {
                    endpoint_path: path,
                },
            );

            let param_info = {
                let s = state.read().unwrap();
                format!(
                    "Confirmed parameter edit (now viewing mode: {})",
                    matches!(s.request.edit_mode, RequestEditMode::Viewing)
                )
            };

            log_debug(&param_info);
        }
    }
}
