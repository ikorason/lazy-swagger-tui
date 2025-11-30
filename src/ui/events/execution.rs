//! Request execution handlers
//!
//! This module handles:
//! - Executing HTTP requests (Space/Enter key)
//! - Expanding/collapsing groups in grouped mode
//! - Retry logic for failed requests

use super::helpers::{can_execute_endpoint, log_debug};
use crate::request::execute_request_background;
use crate::state::{count_visible_items, AppState};
use crate::types::{ApiResponse, RenderItem, ViewMode};
use ratatui::widgets::ListState;
use std::sync::{Arc, RwLock};

/// Handle Enter/Space key - execute request or expand/collapse group
pub fn handle_enter(
    selected_index: &mut usize,
    state: Arc<RwLock<AppState>>,
    list_state: &mut ListState,
    base_url: Option<String>,
) {
    let state_read = state.read().unwrap();

    // Check what view mode we're in
    if state_read.ui.view_mode == ViewMode::Flat {
        // In flat mode: Execute request
        if let Some(endpoint) = state_read.data.endpoints.get(*selected_index) {
            let endpoint = endpoint.clone();

            // Check if we have base_url configured
            if let Some(base_url) = base_url {
                // Check if this endpoint is already executing
                if let Some(ref executing) = state_read.request.executing_endpoint {
                    if executing == &endpoint.path {
                        log_debug("Request already in progress for this endpoint");
                        return;
                    }
                }

                // Validate that all required path params are filled
                let config = state_read.request.configs.get(&endpoint.path);
                if let Err(err_msg) = can_execute_endpoint(&endpoint, config) {
                    log_debug(&format!("Cannot execute: {}", err_msg));
                    drop(state_read);

                    // Store error in response so user can see it
                    let mut s = state.write().unwrap();
                    s.request.current_response = Some(ApiResponse::error(err_msg));
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
        let render_items = state_read.get_render_items();
        if let Some(item) = render_items.get(*selected_index) {
            match item {
                RenderItem::GroupHeader { name, .. } => {
                    let group_name = name.clone();

                    drop(state_read);
                    let mut state_write = state.write().unwrap();

                    if state_write.ui.expanded_groups.contains(&group_name) {
                        state_write.ui.expanded_groups.remove(&group_name);
                        log_debug(&format!("Collapsed group: {}", group_name));
                    } else {
                        state_write.ui.expanded_groups.insert(group_name.clone());
                        log_debug(&format!("Expanded group: {}", group_name));
                    }

                    let visible_count = count_visible_items(&state_write);
                    if *selected_index >= visible_count {
                        *selected_index = visible_count.saturating_sub(1);
                        list_state.select(Some(*selected_index));
                    }
                }
                RenderItem::Endpoint { endpoint } => {
                    let endpoint = endpoint.clone();

                    // Check if we have base_url configured
                    if let Some(base_url) = base_url {
                        // Check if this endpoint is already executing
                        if let Some(ref executing) = state_read.request.executing_endpoint {
                            if executing == &endpoint.path {
                                log_debug("Request already in progress for this endpoint");
                                return;
                            }
                        }

                        // Validate that all required path params are filled
                        let config = state_read.request.configs.get(&endpoint.path);
                        if let Err(err_msg) = can_execute_endpoint(&endpoint, config) {
                            log_debug(&format!("Cannot execute: {}", err_msg));
                            drop(state_read);

                            // Store error in response so user can see it
                            let mut s = state.write().unwrap();
                            s.request.current_response = Some(ApiResponse::error(err_msg));
                            return;
                        }

                        drop(state_read);

                        log_debug(&format!("Executing: {} {}", endpoint.method, endpoint.path));
                        execute_request_background(state.clone(), endpoint, base_url);
                    } else {
                        log_debug("Cannot execute: Base URL not configured");
                    }
                }
            }
        }
    }
}

/// Handle retry after error (Ctrl+R)
pub fn handle_retry(state: Arc<RwLock<AppState>>) -> bool {
    let state_read = state.read().unwrap();
    if matches!(
        state_read.data.loading_state,
        crate::types::LoadingState::Error(_)
    ) {
        drop(state_read);

        // Increment retry count
        if let Ok(mut s) = state.write() {
            s.data.retry_count += 1;
        }

        return true; // Signal that we should fetch
    }
    false // Don't fetch if not in error state
}
