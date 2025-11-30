//! Helper functions for event handling
//!
//! This module contains utility functions used across event handlers:
//! - State locking helpers (apply actions)
//! - Edit mode checking
//! - Validation functions
//! - Paste batching
//! - Debug logging

use crate::actions::{apply_action, AppAction};
use crate::state::AppState;
use crate::types::{ApiEndpoint, RequestConfig, RequestEditMode};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, RwLock};

/// Check if currently editing a parameter
pub fn is_editing(state: &Arc<RwLock<AppState>>) -> bool {
    let state_read = state.read().unwrap();
    matches!(state_read.request.edit_mode, RequestEditMode::Editing(_))
}

/// Apply an action that might depend on edit mode
/// If editing, treat as character input, otherwise apply the action
pub fn apply_or_char(state: Arc<RwLock<AppState>>, ch: char, action: AppAction) {
    if is_editing(&state) {
        let mut s = state.write().unwrap();
        apply_action(AppAction::AppendToParamBuffer(ch.to_string()), &mut s);
    } else {
        let mut s = state.write().unwrap();
        apply_action(action, &mut s);
    }
}

/// Apply a single action to state
pub fn apply(state: Arc<RwLock<AppState>>, action: AppAction) {
    let mut s = state.write().unwrap();
    apply_action(action, &mut s);
}

/// Apply multiple actions to state
pub fn apply_many(state: Arc<RwLock<AppState>>, actions: Vec<AppAction>) {
    let mut s = state.write().unwrap();
    for action in actions {
        apply_action(action, &mut s);
    }
}

/// Check if endpoint can be executed (all required path params are filled)
pub fn can_execute_endpoint(
    endpoint: &ApiEndpoint,
    config: Option<&RequestConfig>,
) -> Result<(), String> {
    // If endpoint has no path parameters, it can always be executed
    let path_params = endpoint.path_params();
    if path_params.is_empty() {
        return Ok(());
    }

    // If we have path params, we need a config
    let config = match config {
        Some(c) => c,
        None => {
            return Err(format!(
                "Please configure path parameter(s): {}",
                path_params
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
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

/// Collect a batch of characters for paste support
///
/// When a character is typed, this function checks for any immediately available
/// character events and batches them together. This enables fast paste operations
/// in terminals.
///
/// Returns a tuple of (batched_string, character_count)
pub fn collect_paste_batch(initial_char: char) -> (String, usize) {
    let mut chars = vec![initial_char];

    // Drain any immediately available character events
    while let Ok(true) = event::poll(std::time::Duration::from_millis(0)) {
        if let Ok(Event::Key(next_key)) = event::read() {
            match next_key.code {
                KeyCode::Char(next_c) if !next_key.modifiers.contains(KeyModifiers::CONTROL) => {
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

    let count = chars.len();
    let batch_str: String = chars.into_iter().collect();
    (batch_str, count)
}

/// Log debug message to /tmp/lazy-swagger-tui.log
pub fn log_debug(msg: &str) {
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/lazy-swagger-tui.log")
        .and_then(|mut f| writeln!(f, "{msg}"));
}
