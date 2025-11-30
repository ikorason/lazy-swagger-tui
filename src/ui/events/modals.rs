//! Modal dialog handlers
//!
//! This module handles user input for modal dialogs:
//! - URL configuration (Swagger URL and Base URL)
//! - Authentication token input
//! - Confirmation dialogs

use super::helpers::{apply, apply_many, collect_paste_batch, log_debug};
use crate::actions::AppAction;
use crate::config;
use crate::state::AppState;
use crate::types::{InputMode, UrlInputField, UrlSubmission};
use color_eyre::Result;
use crossterm::event::KeyCode;
use std::sync::{Arc, RwLock};

/// Handle URL dialog activation
pub fn handle_url_dialog(
    state: Arc<RwLock<AppState>>,
    swagger_url: Option<String>,
    base_url: Option<String>,
) {
    apply(
        state,
        AppAction::EnterUrlInputMode {
            swagger_url,
            base_url,
        },
    );
    log_debug("Entering URL input mode");
}

/// Handle URL input modal (with paste batching support)
pub fn handle_url_input(
    key: crossterm::event::KeyEvent,
    state: Arc<RwLock<AppState>>,
) -> Result<Option<UrlSubmission>> {
    use crossterm::event::KeyModifiers;

    match key.code {
        KeyCode::Tab => {
            // Switch between fields
            let current_field = {
                let s = state.read().unwrap();
                s.input.active_url_field.clone()
            };

            let new_field = match current_field {
                UrlInputField::SwaggerUrl => UrlInputField::BaseUrl,
                UrlInputField::BaseUrl => UrlInputField::SwaggerUrl,
            };

            apply(state.clone(), AppAction::SetActiveUrlField(new_field));
        }

        KeyCode::Enter => {
            let mut s = state.write().unwrap();
            let swagger_url = s.input.url_input.trim().to_string();
            let base_url = s.input.base_url_input.trim().to_string();

            if !swagger_url.is_empty() {
                // Validate both URLs
                match config::validate_url(&swagger_url) {
                    Ok(_) => {
                        // Also validate base URL if provided
                        if !base_url.is_empty() {
                            if let Err(e) = config::validate_url(&base_url) {
                                log_debug(&format!("Invalid base URL: {e}"));
                                // Keep modal open
                                return Ok(None);
                            }
                        }

                        s.input.mode = InputMode::Normal;

                        let submission = UrlSubmission {
                            swagger_url: swagger_url.clone(),
                            base_url: if base_url.is_empty() {
                                None
                            } else {
                                Some(base_url.clone())
                            },
                        };

                        s.input.url_input.clear();
                        s.input.base_url_input.clear();
                        s.input.active_url_field = UrlInputField::SwaggerUrl;

                        log_debug(&format!(
                            "URLs submitted - Swagger: {}, Base: {:?}",
                            submission.swagger_url, submission.base_url
                        ));

                        return Ok(Some(submission));
                    }
                    Err(e) => {
                        log_debug(&format!("Invalid swagger URL: {e}"));
                    }
                }
            } else {
                log_debug("Empty swagger URL, not submitting");
            }
        }

        KeyCode::Esc => {
            apply(state, AppAction::ExitUrlInputMode);
            log_debug("URL input cancelled");
        }

        KeyCode::Backspace => {
            let active_field = {
                let s = state.read().unwrap();
                s.input.active_url_field.clone()
            };

            match active_field {
                UrlInputField::SwaggerUrl => {
                    apply(state, AppAction::BackspaceUrlInput);
                }
                UrlInputField::BaseUrl => {
                    apply(state, AppAction::BackspaceBaseUrlInput);
                }
            }
        }

        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+W: Delete word backwards
            let active_field = {
                let s = state.read().unwrap();
                s.input.active_url_field.clone()
            };

            match active_field {
                UrlInputField::SwaggerUrl => {
                    apply(state, AppAction::DeleteWordUrlInput);
                }
                UrlInputField::BaseUrl => {
                    apply(state, AppAction::DeleteWordBaseUrlInput);
                }
            }
        }

        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+L: Clear current field (matching search behavior)
            let active_field = {
                let s = state.read().unwrap();
                s.input.active_url_field.clone()
            };

            match active_field {
                UrlInputField::SwaggerUrl => {
                    apply(state, AppAction::ClearUrlInput);
                    log_debug("Cleared swagger URL input");
                }
                UrlInputField::BaseUrl => {
                    apply(state, AppAction::ClearBaseUrlInput);
                    log_debug("Cleared base URL input");
                }
            }
        }

        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let (batch_str, char_count) = collect_paste_batch(c);

            let active_field = {
                let s = state.read().unwrap();
                s.input.active_url_field.clone()
            };

            match active_field {
                UrlInputField::SwaggerUrl => {
                    apply(state.clone(), AppAction::AppendToUrlInput(batch_str));
                }
                UrlInputField::BaseUrl => {
                    apply(state.clone(), AppAction::AppendToBaseUrlInput(batch_str));
                }
            }

            if char_count > 1 {
                log_debug(&format!(
                    "Batched {} characters for {}",
                    char_count,
                    if matches!(active_field, UrlInputField::SwaggerUrl) {
                        "Swagger URL"
                    } else {
                        "Base URL"
                    }
                ));
            }
        }

        _ => {}
    }

    Ok(None)
}

/// Handle token input modal (with paste batching support)
pub fn handle_token_input(
    key: crossterm::event::KeyEvent,
    state: Arc<RwLock<AppState>>,
) -> Result<()> {
    use crossterm::event::KeyModifiers;

    match key.code {
        KeyCode::Enter => {
            let token = {
                let s = state.read().unwrap();
                s.input.token_input.trim().to_string()
            };

            if !token.is_empty() {
                apply_many(
                    state,
                    vec![
                        AppAction::SetAuthToken(token),
                        AppAction::ExitTokenInputMode,
                    ],
                );
                log_debug("Token saved");
            } else {
                apply(state, AppAction::ExitTokenInputMode);
                log_debug("Empty token, not saving");
            }
        }
        KeyCode::Esc => {
            apply(state, AppAction::ExitTokenInputMode);
            log_debug("Token input cancelled");
        }
        KeyCode::Backspace => {
            apply(state, AppAction::BackspaceTokenInput);
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            apply(state, AppAction::ClearTokenInput);
            log_debug("Cleared token input");
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            apply(state, AppAction::DeleteWordTokenInput);
        }
        KeyCode::Char(c) => {
            let (batch_str, char_count) = collect_paste_batch(c);

            apply(state, AppAction::AppendToTokenInput(batch_str));

            if char_count > 1 {
                log_debug(&format!(
                    "Batched {char_count} characters (paste detected)"
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle clear token confirmation dialog
pub fn handle_clear_confirmation(
    key: crossterm::event::KeyEvent,
    state: Arc<RwLock<AppState>>,
) -> Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            apply_many(
                state,
                vec![
                    AppAction::ClearAuthToken,
                    AppAction::ExitConfirmClearTokenMode,
                ],
            );
            log_debug("Token cleared");
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            apply(state, AppAction::ExitConfirmClearTokenMode);
            log_debug("Token clear cancelled");
        }
        _ => {}
    }
    Ok(())
}

/// Handle auth dialog activation
pub fn handle_auth_dialog(state: Arc<RwLock<AppState>>) {
    // Pre-fill with current token if exists
    let current_token = {
        let s = state.read().unwrap();
        s.request.auth.token.clone().unwrap_or_default()
    };

    apply_many(
        state,
        vec![
            AppAction::EnterTokenInputMode,
            AppAction::AppendToTokenInput(current_token),
        ],
    );
    log_debug("Entering token input mode");
}

/// Handle body dialog activation
pub fn handle_body_dialog(state: Arc<RwLock<AppState>>, selected_index: usize) {
    // Pre-fill with current body if exists
    let (current_body, endpoint_path) = {
        let s = state.read().unwrap();
        let endpoint = s.get_selected_endpoint(selected_index);
        let path = endpoint.as_ref().map(|ep| ep.path.clone());
        let body = path
            .as_ref()
            .and_then(|p| s.request.configs.get(p))
            .and_then(|c| c.body.clone())
            .unwrap_or_else(|| "{}".to_string());
        (body, path)
    };

    if endpoint_path.is_some() {
        // Set the editor content directly instead of using AppendToBodyInput
        let mut s = state.write().unwrap();
        s.input.body_editor.set_content(current_body.clone());
        s.input.mode = InputMode::EnteringBody;
        log_debug(&format!(
            "Entering body input mode with initial content: {current_body:?}"
        ));
    }
}

/// Handle body input modal (with paste batching and formatting support)
pub fn handle_body_input(
    key: crossterm::event::KeyEvent,
    state: Arc<RwLock<AppState>>,
    selected_index: usize,
) -> Result<()> {
    use crossterm::event::KeyModifiers;

    // Debug: Log all Enter key events
    if matches!(key.code, KeyCode::Enter) {
        log_debug(&format!(
            "Enter key detected - code: {:?}, modifiers: {:?}, has_shift: {}, has_ctrl: {}",
            key.code,
            key.modifiers,
            key.modifiers.contains(KeyModifiers::SHIFT),
            key.modifiers.contains(KeyModifiers::CONTROL)
        ));
    }

    match key.code {
        // Ctrl+N: Insert newline (N for Newline)
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut s = state.write().unwrap();
            s.input.body_validation_error = None;
            s.input.body_editor.insert_newline();
            log_debug("Inserted newline at cursor position (Ctrl+N)");
        }

        KeyCode::Enter => {
            // Enter (without Shift or Ctrl): Save and close
            log_debug(&format!(
                "Enter pressed for save (modifiers: {:?})",
                key.modifiers
            ));
            let state_read = state.read().unwrap();

            // Get the current endpoint path
            let endpoint_path = state_read
                .get_selected_endpoint(selected_index)
                .map(|ep| ep.path.clone());

            drop(state_read);

            if let Some(path) = endpoint_path {
                let mut s = state.write().unwrap();

                // Log the original content before formatting
                let original_body = s.input.body_editor.content().to_string();
                log_debug(&format!("Original body: {original_body}"));

                // Validate JSON before accepting
                let validation_result = s.input.body_editor.validate_json();

                match validation_result {
                    Ok(_) => {
                        // Valid JSON - format and save
                        let _ = s.input.body_editor.format_json();
                        let formatted_body = s.input.body_editor.content().to_string();

                        log_debug(&format!("Formatted JSON successfully: {formatted_body}"));

                        // Save formatted body to config
                        let config = s.get_or_create_request_config_by_path(&path);
                        config.body = if formatted_body.trim().is_empty() {
                            None
                        } else {
                            Some(formatted_body.clone())
                        };

                        log_debug(&format!(
                            "Saved body to config for path '{}': {:?}",
                            path, config.body
                        ));

                        // Close modal and clear error
                        s.input.mode = InputMode::Normal;
                        s.input.body_editor.clear();
                        s.input.body_validation_error = None;

                        log_debug("Body editor modal closed");
                    }
                    Err(e) => {
                        // Invalid JSON - show error and keep modal open
                        s.input.body_validation_error = Some(e.clone());
                        log_debug(&format!(
                            "JSON validation failed: {e}. Keeping modal open."
                        ));
                    }
                }
            }
        }

        KeyCode::Esc => {
            let mut s = state.write().unwrap();
            s.input.mode = InputMode::Normal;
            s.input.body_editor.clear();
            s.input.body_validation_error = None;
            log_debug("Body input cancelled");
        }

        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Use the editor's built-in paste batching
            let mut s = state.write().unwrap();

            // Clear validation error when user starts typing
            s.input.body_validation_error = None;

            let char_count = s.input.body_editor.handle_paste_batch(c);

            if char_count > 1 {
                log_debug(&format!(
                    "Batched {char_count} characters (paste detected)"
                ));

                // Log content before format
                let before_content = s.input.body_editor.content();
                log_debug(&format!(
                    "Content before format (len={}): {}",
                    before_content.len(),
                    before_content
                ));
                log_debug(&format!(
                    "Lines before format: {}",
                    before_content.lines().count()
                ));
                log_debug(&format!(
                    "Cursor before format: {:?}",
                    s.input.body_editor.cursor()
                ));

                // Auto-format JSON on paste
                let format_result = s.input.body_editor.format_json();
                match format_result {
                    Ok(_) => {
                        let after_content = s.input.body_editor.content();
                        log_debug("Auto-formatted pasted JSON successfully");
                        log_debug(&format!(
                            "Content after format (len={}): {}",
                            after_content.len(),
                            after_content
                        ));
                        log_debug(&format!(
                            "Lines after format: {}",
                            after_content.lines().count()
                        ));
                        log_debug(&format!(
                            "Cursor after format: {:?}",
                            s.input.body_editor.cursor()
                        ));

                        // Also log the rendered version
                        let with_cursor = s.input.body_editor.content_with_cursor();
                        log_debug(&format!(
                            "Rendered content (len={}, lines={}): {}",
                            with_cursor.len(),
                            with_cursor.lines().count(),
                            with_cursor
                        ));
                    }
                    Err(e) => {
                        log_debug(&format!(
                            "Auto-format failed (invalid JSON): {e}. Keeping pasted content as-is."
                        ));
                        // Don't show error - just keep the pasted content unformatted
                        // User can fix it and format will happen on Enter
                    }
                }
            }
        }

        _ => {
            // Delegate all other key events to the editor
            let mut s = state.write().unwrap();

            // Clear validation error when user edits
            s.input.body_validation_error = None;

            s.input.body_editor.handle_key_event(key);
        }
    }

    Ok(())
}
