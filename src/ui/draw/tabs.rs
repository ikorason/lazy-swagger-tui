//! Tab rendering for details panel
//!
//! This module contains rendering functions for the detail tabs:
//! - Endpoint tab (method, path, summary, tags)
//! - Request tab (parameters with inline editing)
//! - Headers tab (response headers)
//! - Response tab (response body with JSON formatting)

use super::styling::get_method_color;
use crate::state::AppState;
use crate::types::{ApiEndpoint, ApiParameter, DetailTab, RequestEditMode};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use std::collections::HashMap;

/// Render the Endpoint tab content
pub fn render_endpoint_tab(frame: &mut Frame, area: Rect, endpoint: &ApiEndpoint) {
    let mut lines: Vec<Line> = Vec::new();

    let method_color = get_method_color(&endpoint.method);

    lines.push(Line::from(vec![
        Span::styled(
            &endpoint.method,
            Style::default()
                .fg(method_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(&endpoint.path),
    ]));

    lines.push(Line::from("")); // Empty line

    if let Some(summary) = &endpoint.summary {
        lines.push(Line::from(vec![
            Span::styled("Summary: ", Style::default().fg(Color::Cyan)),
            Span::raw(summary),
        ]));
    }

    if !endpoint.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tags: ", Style::default().fg(Color::Cyan)),
            Span::raw(endpoint.tags.join(", ")),
        ]));
    }

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    frame.render_widget(content, area);
}

/// Render the Request tab content (parameters, etc.)
pub fn render_request_tab(frame: &mut Frame, area: Rect, endpoint: &ApiEndpoint, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();

    // Get path and query parameters for this endpoint
    let path_params: Vec<&ApiParameter> = endpoint.path_params();
    let query_params: Vec<&ApiParameter> = endpoint.query_params();

    // Check if there are ANY parameters or body support
    if path_params.is_empty() && query_params.is_empty() && !endpoint.supports_body() {
        lines.push(Line::from(Span::styled(
            "No parameters defined for this endpoint",
            Style::default().fg(Color::DarkGray),
        )));

        let content = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(content, area);
        return;
    }

    // Get request config for this endpoint
    let config = state.request.configs.get(&endpoint.path);

    // Show helpful message if no parameters but has body support
    if path_params.is_empty() && query_params.is_empty() && endpoint.supports_body() {
        lines.push(Line::from(Span::styled(
            "No parameters defined for this endpoint",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from("")); // Empty line
    }

    let total_path_params = path_params.len();

    // ===== SECTION 1: Path Parameters =====
    if !path_params.is_empty() {
        lines.push(Line::from(Span::styled(
            "Path Parameters:",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from("")); // Empty line

        // Display each path parameter
        for (idx, param) in path_params.iter().enumerate() {
            let global_idx = idx; // Path params come first
            let is_selected = state.ui.selected_param_index == global_idx;

            let current_value =
                if let RequestEditMode::Editing(editing_param_name) = &state.request.edit_mode {
                    if editing_param_name == &param.name {
                        state.request.param_edit_buffer.as_str()
                    } else {
                        config
                            .and_then(|c| c.get_param_value(&param.name))
                            .unwrap_or("")
                    }
                } else {
                    config
                        .and_then(|c| c.get_param_value(&param.name))
                        .unwrap_or("")
                };

            let is_editing = matches!(
                &state.request.edit_mode,
                RequestEditMode::Editing(name) if name == &param.name
            );

            let line = build_param_line(
                param,
                current_value,
                is_selected,
                is_editing,
                true, // is_path_param
            );
            lines.push(line);
        }

        lines.push(Line::from("")); // Empty line after path params
    }

    // ===== SECTION 2: Query Parameters =====
    if !query_params.is_empty() {
        lines.push(Line::from(Span::styled(
            "Query Parameters:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from("")); // Empty line

        // Display each query parameter
        for (idx, param) in query_params.iter().enumerate() {
            let global_idx = total_path_params + idx; // Offset by path param count
            let is_selected = state.ui.selected_param_index == global_idx;

            let current_value =
                if let RequestEditMode::Editing(editing_param_name) = &state.request.edit_mode {
                    if editing_param_name == &param.name {
                        state.request.param_edit_buffer.as_str()
                    } else {
                        config
                            .and_then(|c| c.get_param_value(&param.name))
                            .unwrap_or("")
                    }
                } else {
                    config
                        .and_then(|c| c.get_param_value(&param.name))
                        .unwrap_or("")
                };

            let is_editing = matches!(
                &state.request.edit_mode,
                RequestEditMode::Editing(name) if name == &param.name
            );

            let line = build_param_line(
                param,
                current_value,
                is_selected,
                is_editing,
                false, // is_path_param
            );
            lines.push(line);
        }

        lines.push(Line::from("")); // Empty line after query params
    }

    // ===== SECTION 3: Request Body (for POST/PUT/PATCH) =====
    if endpoint.supports_body() {
        lines.push(Line::from("")); // Empty line

        // Collapsible header
        let expand_icon = if state.ui.body_section_expanded {
            "▼"
        } else {
            "▶"
        };
        let header_text = format!("{expand_icon} Request Body:");

        lines.push(Line::from(vec![
            Span::styled(
                header_text,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "[Press 'b' to edit, 'x' to toggle]",
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        if state.ui.body_section_expanded {
            lines.push(Line::from("")); // Empty line

            // Get current body value
            let body_value = config
                .and_then(|c| c.body.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("{}");

            // Display body (truncate if too long)
            let body_lines: Vec<&str> = body_value.lines().collect();
            let preview_lines = if body_lines.len() > 5 {
                let mut preview = body_lines[..5].to_vec();
                preview.push("  ... (press 'b' to edit)");
                preview
            } else {
                body_lines
            };

            for line in preview_lines {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::Yellow),
                )));
            }
        }

        lines.push(Line::from("")); // Empty line after body
    }

    // ===== SECTION 4: URL Preview =====
    lines.push(Line::from(Span::styled(
        "Preview URL:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    // Build preview URL with both path and query params
    let preview_url = if let Some(config) = config {
        let path_params = config.path_params_map();
        let query_params = config.query_params_map();
        build_preview_url(&endpoint.path, &path_params, &query_params)
    } else {
        endpoint.path.clone()
    };

    lines.push(Line::from(Span::styled(
        preview_url,
        Style::default().fg(Color::Yellow),
    )));

    // ===== SECTION 5: Help Text =====
    lines.push(Line::from("")); // Empty line

    let help_text = match &state.request.edit_mode {
        RequestEditMode::Viewing => {
            if endpoint.supports_body() {
                "j/k/↑/↓: Navigate  |  e: Edit param  |  b: Edit body  |  x: Toggle body  |  Space: Execute"
            } else {
                "j/k/↑/↓: Navigate  |  e: Edit parameter  |  Space: Execute"
            }
        }
        RequestEditMode::Editing(_) => "Type to edit  |  Enter: Confirm  |  Esc: Cancel",
    };

    lines.push(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(content, area);
}

/// Render the Headers tab content
pub fn render_headers_tab(frame: &mut Frame, area: Rect, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref response) = state.request.current_response {
        if !response.headers.is_empty() {
            let mut header_vec: Vec<_> = response.headers.iter().collect();
            header_vec.sort_by_key(|(k, _)| k.as_str());

            for (key, value) in header_vec {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", key), Style::default().fg(Color::Cyan)),
                    Span::raw(value.to_string()),
                ]));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "No headers",
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No response yet",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });

    frame.render_widget(content, area);
}

/// Render the Response tab content
pub fn render_response_tab(
    frame: &mut Frame,
    area: Rect,
    endpoint: &ApiEndpoint,
    state: &AppState,
) {
    let mut lines: Vec<Line> = Vec::new();

    let is_executing = state.request.executing_endpoint.as_ref() == Some(&endpoint.path);

    if is_executing {
        lines.push(Line::from(vec![Span::styled(
            "⏳ Executing request...",
            Style::default().fg(Color::Cyan),
        )]));
    } else if let Some(ref response) = state.request.current_response {
        if response.is_error {
            lines.push(Line::from(vec![Span::styled(
                "❌ Error",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            if let Some(ref err_msg) = response.error_message {
                for line in err_msg.lines() {
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::Red),
                    )));
                }
            }
        } else {
            // Show status line
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{} {}", response.status, response.status_text),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("  "),
                Span::styled("Duration: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}ms", response.duration.as_millis())),
            ]));
            lines.push(Line::from("")); // Empty line

            // Show formatted body
            let formatted_body = try_format_json(&response.body);
            for (idx, line) in formatted_body.lines().enumerate() {
                // Highlight selected line when in Response tab
                // response_selected_line counts from 0 including header (status=0, empty=1, body starts at 2)
                let total_line_idx = idx + 2; // Add 2 for status and empty line
                let line_style = if state.ui.active_detail_tab == DetailTab::Response
                    && state.ui.response_selected_line == total_line_idx
                {
                    // Flash green if yank just happened, otherwise gray
                    if state.ui.yank_flash {
                        Style::default()
                            .bg(Color::Green)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().bg(Color::DarkGray)
                    }
                } else {
                    Style::default()
                };
                lines.push(Line::from(Span::styled(line.to_string(), line_style)));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Press [Space] to execute request",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let content = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.ui.response_scroll as u16, 0));

    frame.render_widget(content, area);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build URL preview with path and query parameters
fn build_preview_url(
    path_template: &str,
    path_params: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
) -> String {
    // Step 1: Substitute path parameters
    let mut path = path_template.to_string();

    for (key, value) in path_params {
        let placeholder = format!("{{{}}}", key);
        if path.contains(&placeholder) {
            // Show placeholder if value is empty, otherwise substitute
            if value.is_empty() {
                // Keep the placeholder visible
            } else {
                path = path.replace(&placeholder, value);
            }
        }
    }

    // Step 2: Add query parameters
    let non_empty_params: Vec<String> = query_params
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    if non_empty_params.is_empty() {
        path
    } else {
        format!("{}?{}", path, non_empty_params.join("&"))
    }
}

/// Helper function to build a single parameter line with styling
fn build_param_line(
    param: &ApiParameter,
    current_value: &str,
    is_selected: bool,
    is_editing: bool,
    is_path_param: bool,
) -> Line<'static> {
    // Build type info string (e.g., "integer/int32" or "boolean")
    let type_info = if let Some(schema) = &param.schema {
        let type_str = schema.param_type.as_deref().unwrap_or("unknown");
        if let Some(format) = &schema.format {
            format!("{}/{}", type_str, format)
        } else {
            type_str.to_string()
        }
    } else {
        "unknown".to_string()
    };

    // Build required indicator
    let required_str = if param.required.unwrap_or(false) {
        "*"
    } else {
        ""
    };

    // Selection indicator
    let indicator = if is_selected { "→ " } else { "  " };

    // Value display - show cursor if editing
    let value_display = if is_editing {
        format!("[{}▊]", current_value) // Show cursor
    } else if current_value.is_empty() {
        "[_____]".to_string() // Empty placeholder
    } else {
        format!("[{}]", current_value)
    };

    // Build the line with appropriate styling
    let indicator_style = if is_selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let name_style = if is_selected {
        Style::default()
            .fg(if is_path_param {
                Color::Magenta
            } else {
                Color::Cyan
            })
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(if is_path_param {
            Color::Rgb(180, 100, 180) // Dimmed magenta
        } else {
            Color::White
        })
    };

    let value_style = if is_editing {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };

    let meta_style = Style::default().fg(Color::DarkGray);

    Line::from(vec![
        Span::styled(indicator, indicator_style),
        Span::styled(format!("{}{}: ", param.name, required_str), name_style),
        Span::styled(value_display, value_style),
        Span::raw("  "),
        Span::styled(format!("({})", type_info), meta_style),
    ])
}

/// Attempts to pretty-print JSON, returns original string if not valid JSON
pub fn try_format_json(body: &str) -> String {
    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(json) => {
            // Successfully parsed, pretty-print it
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| body.to_string())
        }
        Err(_) => {
            // Not valid JSON, return as-is
            body.to_string()
        }
    }
}
