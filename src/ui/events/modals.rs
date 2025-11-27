//! Modal dialog handlers
//!
//! This module handles user input for modal dialogs:
//! - URL configuration (Swagger URL and Base URL)
//! - Authentication token input
//! - Confirmation dialogs

use super::helpers::{apply, apply_many, log_debug};
use crate::actions::AppAction;
use crate::config;
use crate::state::AppState;
use crate::types::{InputMode, UrlInputField, UrlSubmission};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
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
            let mut s = state.write().unwrap();

            match s.input.active_url_field {
                UrlInputField::SwaggerUrl => {
                    s.input.active_url_field = UrlInputField::BaseUrl;
                }
                UrlInputField::BaseUrl => {
                    s.input.active_url_field = UrlInputField::SwaggerUrl;
                }
            }
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
                                log_debug(&format!("Invalid base URL: {}", e));
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
                        log_debug(&format!("Invalid swagger URL: {}", e));
                    }
                }
            } else {
                log_debug("Empty swagger URL, not submitting");
            }
        }

        KeyCode::Esc => {
            let mut s = state.write().unwrap();
            s.input.mode = InputMode::Normal;
            s.input.url_input.clear();
            s.input.base_url_input.clear();
            s.input.active_url_field = UrlInputField::SwaggerUrl;
            log_debug("URL input cancelled");
        }

        KeyCode::Backspace => {
            let mut s = state.write().unwrap();
            match s.input.active_url_field {
                UrlInputField::SwaggerUrl => {
                    s.input.url_input.pop();
                }
                UrlInputField::BaseUrl => {
                    s.input.base_url_input.pop();
                }
            }
        }

        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+U: Clear entire line
            let mut s = state.write().unwrap();
            match s.input.active_url_field {
                UrlInputField::SwaggerUrl => {
                    s.input.url_input.clear();
                    log_debug("Cleared swagger URL input");
                }
                UrlInputField::BaseUrl => {
                    s.input.base_url_input.clear();
                    log_debug("Cleared base URL input");
                }
            }
        }

        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+W: Delete word backwards
            let mut s = state.write().unwrap();
            let input = match s.input.active_url_field {
                UrlInputField::SwaggerUrl => &mut s.input.url_input,
                UrlInputField::BaseUrl => &mut s.input.base_url_input,
            };

            // Find last word boundary (space, slash, colon, dot)
            if let Some(pos) = input.rfind(|c: char| c == ' ' || c == '/' || c == ':' || c == '.') {
                input.truncate(pos);
            } else {
                input.clear();
            }
        }

        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+L: Clear current field (matching search behavior)
            let mut s = state.write().unwrap();
            match s.input.active_url_field {
                UrlInputField::SwaggerUrl => {
                    s.input.url_input.clear();
                    log_debug("Cleared swagger URL input");
                }
                UrlInputField::BaseUrl => {
                    s.input.base_url_input.clear();
                    log_debug("Cleared base URL input");
                }
            }
        }

        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Collect this character and any pending characters (for paste support)
            let mut chars = vec![c];

            // Drain any immediately available character events
            loop {
                match event::poll(std::time::Duration::from_millis(0)) {
                    Ok(true) => {
                        if let Ok(Event::Key(next_key)) = event::read() {
                            match next_key.code {
                                KeyCode::Char(next_c)
                                    if !next_key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
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
                    _ => break,
                }
            }

            // Log before consuming chars
            let char_count = chars.len();

            // Add all batched characters at once
            let mut s = state.write().unwrap();
            for ch in chars {
                match s.input.active_url_field {
                    UrlInputField::SwaggerUrl => {
                        s.input.url_input.push(ch);
                    }
                    UrlInputField::BaseUrl => {
                        s.input.base_url_input.push(ch);
                    }
                }
            }

            if char_count > 1 {
                log_debug(&format!(
                    "Batched {} characters for {}",
                    char_count,
                    if matches!(s.input.active_url_field, UrlInputField::SwaggerUrl) {
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
            let mut s = state.write().unwrap();
            let token = s.input.token_input.trim().to_string();

            if !token.is_empty() {
                s.request.auth.set_token(token);
                log_debug("Token saved");
            } else {
                log_debug("Empty token, not saving");
            }
            s.input.mode = InputMode::Normal;
            s.input.token_input.clear();
        }
        KeyCode::Esc => {
            let mut s = state.write().unwrap();
            s.input.mode = InputMode::Normal;
            s.input.token_input.clear();
            log_debug("Token input cancelled");
        }
        KeyCode::Backspace => {
            let mut s = state.write().unwrap();
            s.input.token_input.pop();
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+U: Clear entire token
            let mut s = state.write().unwrap();
            s.input.token_input.clear();
            log_debug("Cleared token input");
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+W: Delete word backwards (less useful for tokens, but consistent)
            let mut s = state.write().unwrap();
            if let Some(pos) = s.input.token_input.rfind(|c: char| !c.is_alphanumeric()) {
                s.input.token_input.truncate(pos);
            } else {
                s.input.token_input.clear();
            }
        }
        KeyCode::Char(c) => {
            // Collect this character and any pending characters (for paste support)
            let mut chars = vec![c];

            // Drain any immediately available character events
            loop {
                match event::poll(std::time::Duration::from_millis(0)) {
                    Ok(true) => {
                        if let Ok(Event::Key(next_key)) = event::read() {
                            match next_key.code {
                                KeyCode::Char(next_c)
                                    if !next_key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    chars.push(next_c);
                                }
                                _ => {
                                    // Non-character or control key, stop batching and handle it next iteration
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }

            // Log before consuming chars
            let char_count = chars.len();

            // Add all batched characters at once
            let mut s = state.write().unwrap();
            for ch in chars {
                s.input.token_input.push(ch);
            }

            if char_count > 1 {
                log_debug(&format!(
                    "Batched {} characters (paste detected)",
                    char_count
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
            let mut s = state.write().unwrap();
            s.request.auth.clear_token();
            s.input.mode = InputMode::Normal;
            log_debug("Token cleared");
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            let mut s = state.write().unwrap();
            s.input.mode = InputMode::Normal;
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
