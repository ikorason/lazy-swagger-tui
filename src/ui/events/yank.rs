//! Yank (copy) handlers
//!
//! This module handles copying content to the system clipboard.
//! Currently supports line-based yanking from Response tab.

use super::helpers::log_debug;
use crate::state::AppState;
use crate::ui::draw::try_format_json;
use arboard::Clipboard;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Yank the currently selected line from the Response tab to clipboard
pub fn handle_yank_response_line(state: Arc<RwLock<AppState>>) {
    log_debug("=== Yank handler called ===");
    let state_read = state.read().unwrap();

    // Get the response if available
    if let Some(ref response) = state_read.request.current_response {
        if response.is_error {
            log_debug("Cannot yank from error response");
            return;
        }

        // Get formatted body
        let formatted_body = try_format_json(&response.body);
        let lines: Vec<&str> = formatted_body.lines().collect();

        // The selected line index includes the status line (2 lines at top)
        let selected_line_idx = state_read.ui.response_selected_line;

        log_debug(&format!(
            "Selected line idx: {}, Total body lines: {}, Total lines with header: {}",
            selected_line_idx,
            lines.len(),
            lines.len() + 2
        ));

        // Lines in response: [Status line, Empty line, ...body lines...]
        // If selected_line < 2, we're on the header
        if selected_line_idx < 2 {
            log_debug(&format!("Cannot yank header lines (idx={})", selected_line_idx));
            return;
        }

        // Adjust index to get actual body line
        let body_line_idx = selected_line_idx - 2;

        if body_line_idx >= lines.len() {
            log_debug(&format!(
                "Line index {} out of bounds (body has {} lines)",
                body_line_idx,
                lines.len()
            ));
            return;
        }

        let line_content = lines[body_line_idx];
        log_debug(&format!("Line content: '{}'", line_content));

        // Try to extract just the value if this is a JSON key-value pair
        let value_to_copy = extract_json_value(line_content);
        log_debug(&format!("Extracted value: '{}'", value_to_copy));

        drop(state_read);

        // Copy to clipboard
        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(value_to_copy.clone()) {
                Ok(_) => {
                    log_debug(&format!("✓ Successfully yanked: {}", value_to_copy));

                    // Set flash flag
                    {
                        let mut state_write = state.write().unwrap();
                        state_write.ui.yank_flash = true;
                    }

                    // Spawn task to clear flash after delay
                    let state_clone = state.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                        let mut s = state_clone.write().unwrap();
                        s.ui.yank_flash = false;
                    });
                }
                Err(e) => {
                    log_debug(&format!("✗ Failed to copy to clipboard: {}", e));
                }
            },
            Err(e) => {
                log_debug(&format!("✗ Failed to access clipboard: {}", e));
            }
        }
    } else {
        log_debug("No response available to yank");
    }
}

/// Extract the value portion from a JSON line
/// Examples:
///   "  "access_token": "abc123"," -> "abc123"
///   "  \"name\": \"John\"" -> "John"
///   "  123" -> "123"
fn extract_json_value(line: &str) -> String {
    let trimmed = line.trim();

    // Check if line contains a colon (key-value pair)
    if let Some(colon_pos) = trimmed.find(':') {
        // Extract everything after the colon
        let value_part = &trimmed[colon_pos + 1..];

        // Remove leading/trailing whitespace, quotes, and trailing comma
        let cleaned = value_part
            .trim()
            .trim_end_matches(',')
            .trim()
            .trim_matches('"');

        cleaned.to_string()
    } else {
        // Not a key-value pair, return the trimmed line (remove brackets, commas, etc.)
        trimmed
            .trim_matches(|c| c == '{' || c == '}' || c == '[' || c == ']' || c == ',')
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_value_simple() {
        assert_eq!(
            extract_json_value("  \"access_token\": \"abc123\","),
            "abc123"
        );
        assert_eq!(extract_json_value("  \"name\": \"John\""), "John");
    }

    #[test]
    fn test_extract_json_value_number() {
        assert_eq!(extract_json_value("  \"age\": 30,"), "30");
        assert_eq!(extract_json_value("  \"count\": 123"), "123");
    }

    #[test]
    fn test_extract_json_value_boolean() {
        assert_eq!(extract_json_value("  \"active\": true,"), "true");
        assert_eq!(extract_json_value("  \"enabled\": false"), "false");
    }

    #[test]
    fn test_extract_json_value_no_colon() {
        assert_eq!(extract_json_value("  123"), "123");
        assert_eq!(extract_json_value("  {"), "");
    }
}
