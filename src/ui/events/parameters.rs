//! Parameter editing handlers
//!
//! This module handles editing of request parameters:
//! - Entering edit mode for a selected parameter
//! - Confirming parameter edits
//! - Ensuring request configs exist

use super::helpers::log_debug;
use crate::state::AppState;
use crate::types::{RequestConfig, RequestEditMode};
use std::sync::{Arc, RwLock};

/// Enter edit mode for the currently selected parameter
pub fn handle_request_param_edit(selected_index: usize, state: Arc<RwLock<AppState>>) {
    // First, gather all the data we need while holding read lock
    let edit_data = {
        let state_read = state.read().unwrap();

        // Only enter edit mode if currently in Viewing mode
        if !matches!(state_read.request_edit_mode, RequestEditMode::Viewing) {
            return;
        }

        // Get currently selected endpoint
        let selected_endpoint = state_read.get_selected_endpoint(selected_index);

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

/// Confirm parameter edit and save the value
pub fn handle_request_param_confirm(selected_index: usize, state: Arc<RwLock<AppState>>) {
    let state_read = state.read().unwrap();

    // Get the param name we're editing
    if let RequestEditMode::Editing(param_name) = &state_read.request_edit_mode {
        let param_name = param_name.clone();
        let new_value = state_read.param_edit_buffer.clone();

        // Get currently selected endpoint
        let selected_endpoint = state_read.get_selected_endpoint(selected_index);

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
